//! Thin project timeline: all persistent actions dispatch native commands.
use crate::PhotocraftApp;
use crate::theme::Tokens;
use egui::{Color32, RichText, Sense, Stroke, StrokeKind, vec2};
use photocraft_doc::{DocId, Document};
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, mpsc},
};

type PreviewResult = Result<(Document, photocraft_compose::Buffer), String>;
struct PreviewJob {
    document: DocId,
    monitor: u64,
    hash: String,
    receiver: mpsc::Receiver<PreviewResult>,
}
#[derive(Default)]
pub struct State {
    label: String,
    pub(super) base: String,
    parameters: String,
    parameters_open: bool,
    status: String,
    document: Option<DocId>,
    monitor: u64,
    previews: HashMap<String, Result<egui::TextureHandle, String>>,
    order: VecDeque<String>,
    worker: Option<PreviewJob>,
    pub(super) compare: bool,
    pub(super) comparison: crate::pipeline_compare::State,
    cached_bytes: usize,
    budget_bytes: usize,
}

pub fn inspect(state: &State) -> serde_json::Value {
    json!({"selectedVersion":state.base,"comparing":state.compare,"loadingPreview":state.worker.is_some(),"cachedPreviews":state.previews.values().filter(|r|r.is_ok()).count(),"previewBytes":state.cached_bytes,"previewBudgetBytes":state.budget_bytes,"detailCompare":state.comparison.inspect(),"parametersOpen":state.parameters_open})
}

/// Read-only view controls; persistent edits still dispatch project commands.
pub(crate) fn control(app: &mut PhotocraftApp, ctx: &egui::Context, params: &serde_json::Value) -> Result<serde_json::Value,String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct View { parameters:Option<bool>, version:Option<String>, compare:Option<bool>, zoom:Option<f32>, center:Option<[f32;2]>, difference:Option<bool>, operation:Option<usize> }
    let v:View=serde_json::from_value(params.clone()).map_err(|e|e.to_string())?;
    let st=app.session.active().ok_or("No document")?;
    let p=st.pipeline.as_ref().ok_or("No project history")?;
    if v.version.as_ref().is_some_and(|id|!p.versions.iter().any(|x|&x.id==id)){return Err("Unknown version".into());}
    if v.zoom.is_some_and(|z|!z.is_finite()||!(0.0..=16.0).contains(&z)||(z>0.0&&z<0.01)){return Err("zoom must be 0 (fit) or 0.01..16".into());}
    if v.center.is_some_and(|c|c.iter().any(|x|!x.is_finite())||c[0]<0.0||c[1]<0.0||c[0]>st.doc.size.width as f32||c[1]>st.doc.size.height as f32){return Err("center is outside canvas".into());}
    let recipe=v.operation.map(|i|p.operations.get(i).filter(|op|op.replayable).ok_or("Operation is missing or cannot be replayed").and_then(|op|serde_json::to_string_pretty(&json!([{"command":op.command,"params":op.params,"active_layer":op.active_layer,"selected_layers":op.selected_layers}])).map_err(|_|"Cannot encode recipe"))).transpose()?;
    synchronize(app,ctx);
    let state=&mut app.coskit_ai.project_ui;
    if let Some(id)=v.version{state.base=id;}
    if let Some(open)=v.parameters{state.parameters_open=open;}
    if let Some(open)=v.compare{state.compare=open;if !open{state.comparison.reset();}}
    if let Some(zoom)=v.zoom{state.comparison.zoom=zoom;}
    if let Some(center)=v.center{state.comparison.center=Some(center);}
    if let Some(difference)=v.difference{state.comparison.difference=difference;}
    if let Some(recipe)=recipe{state.parameters=recipe;}
    Ok(inspect(state))
}

/// Decode one immutable version away from the UI. Retain only colour metadata and a proxy.
fn render_preview(bytes: &[u8]) -> PreviewResult {
    let opts = photocraft_format::LoadOptions { max_manifest_bytes: 32 << 20, max_total_bytes: 2 << 30, ..Default::default() };
    let doc = photocraft_format::load_from_bytes_with(bytes, &opts).map_err(|e| e.to_string())?;
    if doc.pipeline.is_some() {
        return Err("不支持嵌套的版本快照".into());
    }
    let buffer = photocraft_compose::thumbnail_buffer(&doc, 768);
    let mut metadata = Document::new("Version preview", doc.size, doc.mode, doc.depth);
    metadata.icc_profile = doc.icc_profile.clone();
    Ok((metadata, buffer))
}

fn render_project_preview(project: &photocraft_doc::pipeline::Pipeline, hash: &str) -> PreviewResult {
    if let Some(bytes) = project.previews.get(hash) {
        let (meta, px) = photocraft_format::preview::decode(bytes).map_err(|e| e.to_string())?;
        let mut doc = Document::new("Version preview", photocraft_geom::Size::new(meta.width, meta.height), meta.mode, meta.depth);
        doc.icc_profile = meta.icc.map(Arc::new);
        return Ok((doc, photocraft_compose::Buffer { rect: photocraft_geom::Rect::from_xywh(0, 0, meta.width, meta.height), px }));
    }
    // Older files do not have embedded proxies. Decode a single requested version in the worker.
    if project.schema == 1 {
        return render_preview(project.snapshots.get(hash).ok_or("Missing version")?);
    }
    let doc = photocraft_format::pipeline::load_snapshot(project, hash).map_err(|e| e.to_string())?;
    let buffer = photocraft_compose::thumbnail_buffer(&doc, 384);
    let mut metadata = Document::new("Version preview", doc.size, doc.mode, doc.depth);
    metadata.icc_profile = doc.icc_profile.clone();
    Ok((metadata, buffer))
}

fn trim_previews(state: &mut State) {
    state.cached_bytes = state.previews.values().filter_map(|v| v.as_ref().ok()).map(|t| t.size()[0].saturating_mul(t.size()[1]).saturating_mul(4)).sum();
    while state.cached_bytes > state.budget_bytes || state.order.len() > 24 {
        let Some(old) = state.order.pop_front() else { break };
        if let Some(Ok(texture)) = state.previews.remove(&old) {
            state.cached_bytes = state.cached_bytes.saturating_sub(texture.size()[0].saturating_mul(texture.size()[1]).saturating_mul(4));
        }
    }
}

fn synchronize(app: &mut PhotocraftApp, ctx: &egui::Context) {
    let document = app.session.active().map(|s| s.doc.id);
    let monitor = app.session.color.monitor().content_hash();
    let state = &mut app.coskit_ai.project_ui;
    if !state.compare {state.comparison.reap_closed();}
    state.budget_bytes = app.session.prefs().performance.preview_budget_bytes();
    trim_previews(state);
    if state.document != document || state.monitor != monitor {
        let switched = state.document != document;
        state.document = document;
        state.monitor = monitor;
        state.cached_bytes = 0;
        state.previews.clear();
        state.order.clear();
        if switched {
            state.base = app.session.active().and_then(|s| s.pipeline.as_ref()).and_then(|p| p.head.clone()).unwrap_or_default();
            state.parameters.clear();
            state.parameters_open = false;
            state.label.clear();
            state.status.clear();
            state.compare = false;
            state.comparison.reset();
        }
    }
    let done = state.worker.as_ref().and_then(|job| match job.receiver.try_recv() {
        Ok(result) => Some(result),
        Err(mpsc::TryRecvError::Disconnected) => Some(Err("预览任务已停止，可重新打开工程重试".into())),
        Err(mpsc::TryRecvError::Empty) => None,
    });
    if let Some(result) = done
        && let Some(job) = state.worker.take()
        && Some(job.document) == document
        && job.monitor == monitor
    {
        let texture = result.and_then(|(doc, buffer)| {
            let display = app.session.color.canvas_display(&doc).map_err(|e| e.to_string())?;
            let rgba = display.to_rgba8(&buffer);
            let image = egui::ColorImage::from_rgba_unmultiplied([rgba.width as usize, rgba.height as usize], &rgba.pixels);
            Ok(ctx.load_texture(format!("ckpipe-{}", job.hash), image, egui::TextureOptions::LINEAR))
        });
        state.order.push_back(job.hash.clone());
        state.previews.insert(job.hash, texture);
        trim_previews(state);
    }
    if state.worker.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(80));
    }
}

fn request_preview(state: &mut State, ctx: &egui::Context, hash: &str, project: Arc<photocraft_doc::pipeline::Pipeline>) {
    if state.worker.is_some() || state.previews.contains_key(hash) {
        return;
    }
    let Some(document) = state.document else {
        return;
    };
    let (tx, rx) = mpsc::channel();
    state.worker = Some(PreviewJob { document, monitor: state.monitor, hash: hash.into(), receiver: rx });
    let ctx = ctx.clone();
    let hash = hash.to_string();
    let work = move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render_project_preview(&project, &hash)))
            .unwrap_or_else(|_| Err("此版本无法生成预览；工程未被修改".into()));
        let _ = tx.send(result);
        ctx.request_repaint();
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::thread::Builder::new().name("ckpipe-preview".into()).spawn(work);
    }
    #[cfg(target_arch = "wasm32")]
    work();
}

fn fit_image(ui: &egui::Ui, rect: egui::Rect, texture: &egui::TextureHandle) {
    let size = texture.size_vec2();
    let scale = (rect.width() / size.x.max(1.0)).min(rect.height() / size.y.max(1.0));
    let target = egui::Rect::from_center_size(rect.center(), size * scale);
    crate::widgets::checker(ui.painter(), target, 6.0);
    ui.painter().image(texture.id(), target, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
}

pub fn timeline(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    synchronize(app, ui.ctx());
    let t = Tokens::get(ui.ctx());
    let project = app.session.active().and_then(|s| s.pipeline.clone());
    let busy = app.coskit_ai.receiver.is_some() || app.session.active_job().is_some();
    let mut action = None;
    egui::Panel::bottom("coskit-version-strip").exact_size(160.0).frame(egui::Frame::NONE.fill(t.dock).inner_margin(egui::Margin::symmetric(16, 10))).show(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                let roomy = ui.available_width() >= 620.0;
                ui.strong("版本");
                let count = project.as_ref().map_or(0, |p| p.versions.len());
                if roomy {
                    ui.label(RichText::new(format!("{count} 个记录")).small().color(t.text_dim));
                }
                if roomy {
                    ui.add(egui::TextEdit::singleline(&mut app.coskit_ai.project_ui.label).hint_text("命名这个阶段…").desired_width(130.0));
                }
                if ui.add_enabled(!busy && app.session.active().is_some(), egui::Button::new("＋ 记录")).clicked() {
                    action = Some((
                        "coskit.project.checkpoint",
                        json!({"label":if app.coskit_ai.project_ui.label.trim().is_empty() { "手动版本" } else { &app.coskit_ai.project_ui.label }}),
                    ));
                }
                let selected = project.as_ref().is_some_and(|p| p.versions.iter().any(|v| v.id == app.coskit_ai.project_ui.base));
                if ui.add_enabled(selected, egui::Button::new("对比")).clicked() {
                    app.coskit_ai.project_ui.compare = true;
                }
                if ui.add_enabled(selected, egui::Button::new("参数")).clicked() { app.coskit_ai.project_ui.parameters_open = true; }
                if ui.add_enabled(selected && !busy, egui::Button::new("恢复所选版本")).clicked() {
                    action = Some(("coskit.project.restore", json!({"version":app.coskit_ai.project_ui.base})));
                }
                if ui.small_button("收起").clicked() {
                    app.ui.studio.versions = false;
                }
            });
            ui.add_space(8.0);
            if let Some(p) = &project
                && !p.versions.is_empty()
            {
                egui::ScrollArea::horizontal().id_salt("coskit-version-cards").auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for version in &p.versions {
                            let state = &mut app.coskit_ai.project_ui;
                            let selected = state.base == version.id;
                            let (rect, response) = ui.allocate_exact_size(vec2(132.0, 90.0), Sense::click());
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("选择版本 {} {}", version.id, version.label))
                            });
                            if ui.is_rect_visible(rect) {
                                ui.painter().rect_filled(rect, 6.0, if selected { t.row_selected } else { t.card });
                                ui.painter().rect_stroke(rect, 6.0, Stroke::new(1.0, if selected { t.accent } else { t.card_border }), StrokeKind::Inside);
                                let image_rect = egui::Rect::from_min_size(rect.min + vec2(5.0, 5.0), vec2(122.0, 59.0));
                                if let Some(Ok(texture)) = state.previews.get(&version.snapshot) {
                                    fit_image(ui, image_rect, texture);
                                    state.order.retain(|hash| hash != &version.snapshot);
                                    state.order.push_back(version.snapshot.clone());
                                } else {
                                    let message =
                                        if state.previews.get(&version.snapshot).is_some_and(Result::is_err) { "预览不可用" } else { "加载预览…" };
                                    ui.painter().text(
                                        image_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        message,
                                        egui::FontId::proportional(11.0),
                                        t.text_faint,
                                    );
                                    request_preview(state, ui.ctx(), &version.snapshot, p.clone());
                                }
                                let label = format!("{}  {}", version.id, if p.head.as_ref() == Some(&version.id) { "当前基准" } else { &version.label });
                                let mut job = egui::text::LayoutJob::simple_singleline(label, egui::FontId::proportional(11.5), t.text);
                                job.wrap = egui::text::TextWrapping::truncate_at_width(122.0);
                                ui.painter().galley(rect.min + vec2(5.0, 70.0), ui.painter().layout_job(job), t.text);
                            }
                            if response.clicked() {
                                state.base = version.id.clone();
                            }
                            response.on_hover_text(format!(
                                "{} · {}\n来自 {} · {} 条编辑\n点击选择，对比后可恢复",
                                version.id,
                                version.label,
                                version.parent.as_deref().unwrap_or("起点"),
                                version.operation_count
                            ));
                        }
                    });
                });
            } else {
                ui.add_space(18.0);
                ui.label(RichText::new("把满意的阶段记下来，随时对比，随时从这里继续。").color(t.text_dim));
                ui.label(RichText::new("保存 .ckpipe 会一起保留图层、对话和版本。").small().color(t.text_faint));
            }
        },
    );
    if let Some((command, params)) = action {
        match app.run(command, params) {
            Ok(v) => {
                if let Some(id) = v.get("version").and_then(serde_json::Value::as_str) {
                    app.coskit_ai.project_ui.base = id.into();
                }
                app.ui.status = if v.get("pending").and_then(serde_json::Value::as_bool)==Some(true) {
                    "任务已开始，完成后更新版本；可在进度栏取消"
                } else if command == "coskit.project.restore" {
                    "已恢复所选版本，原分支已保留；可撤销"
                } else {
                    "版本已记录；保存 .ckpipe 后可在下次打开时恢复"
                }
                .into();
                app.ui.status_error = false;
            }
            Err(e) => {
                app.ui.status = e;
                app.ui.status_error = true;
            }
        }
    }
}

pub fn compare_window(app: &mut PhotocraftApp, ctx: &egui::Context) {
    synchronize(app, ctx);
    parameter_window(app, ctx);
    if !app.coskit_ai.project_ui.compare {
        return;
    }
    crate::pipeline_compare::window(app, ctx);
}

fn parameter_window(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if !app.coskit_ai.project_ui.parameters_open {return;}
    let Some(st)=app.session.active() else {return};
    let Some(project)=st.pipeline.clone() else {return};
    let mode=st.doc.mode;
    let busy=app.coskit_ai.receiver.is_some()||app.session.active_job().is_some();
    let mut open=true;let mut replay=None;
    egui::Window::new("版本参数").open(&mut open).default_width(430.0).resizable(true).show(ctx,|ui|{
        ui.label("调整配方后在所选基准上生成新分支，原版本保留。");
        let state=&mut app.coskit_ai.project_ui;
        egui::ComboBox::from_label("基准版本").selected_text(&state.base).show_ui(ui,|ui|{for v in &project.versions{ui.selectable_value(&mut state.base,v.id.clone(),format!("{} · {}",v.id,v.label));}});
        egui::ComboBox::from_label("编辑记录").selected_text("选择一条可重放操作…").show_ui(ui,|ui|{
            for (index,op) in project.operations.iter().enumerate().rev().take(200){if ui.add_enabled(op.replayable,egui::Button::new(format!("{} · {}",index+1,op.command))).clicked(){
                state.parameters=serde_json::to_string_pretty(&json!([{"command":op.command,"params":op.params,"active_layer":op.active_layer,"selected_layers":op.selected_layers}])).unwrap_or_default();
            }}
        });
        egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-320.0).max(120.0)).show(ui,|ui|{
            crate::pipeline_parameters::editor(ui,&mut state.parameters,mode);
            ui.collapsing("高级 JSON",|ui|{ui.add(egui::TextEdit::multiline(&mut state.parameters).desired_rows(6).desired_width(f32::INFINITY).code_editor());});
        });
        if ui.add_enabled(!busy&&!state.parameters.is_empty(),egui::Button::new("重放并生成新版本")).clicked(){
            match serde_json::from_str::<serde_json::Value>(&state.parameters){Ok(steps)=>replay=Some(json!({"version":state.base,"steps":steps})),Err(e)=>state.status=e.to_string()}
        }
        ui.label(&state.status);
    });
    app.coskit_ai.project_ui.parameters_open=open;
    if let Some(params)=replay{app.coskit_ai.project_ui.status=match app.run("coskit.project.replay",params){Ok(v) if v.get("pending").and_then(serde_json::Value::as_bool)==Some(true)=>"正在后台重放，可在进度栏取消".into(),Ok(_)=>"已生成新版本".into(),Err(e)=>e};}
}

pub fn panel(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    ui.collapsing("CKPipe 工程 · 版本与参数", |ui| {
        ui.label("保存为 .ckpipe 可保留图层、对话、参数记录与版本。恢复旧版本会保留当前版本；参数重放在副本中执行。");
        let project = app.session.active().and_then(|s| s.pipeline.clone());
        let mut action = None;
        let busy = app.coskit_ai.receiver.is_some() || app.session.active_job().is_some();
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.coskit_ai.project_ui.label).hint_text("版本名称").desired_width(120.0));
                if ui.button("记录版本").clicked() {
                    action = Some(("coskit.project.checkpoint", json!({"label":app.coskit_ai.project_ui.label})));
                }
                if ui.button("保存工程…").clicked() {
                    let result = app.pick_save("CosKit.ckpipe", |app, path| app.save_as(Some(path)));
                    if let Err(e) = result {
                        app.coskit_ai.project_ui.status = e;
                    }
                }
            });
            if let Some(p) = &project {
                ui.label(format!("{} 个版本 · {} 条编辑记录 · {} 次工作流", p.versions.len(), p.operations.len(), p.runs.len()));
                if p.omitted_operations > 0 {
                    ui.label(format!("{} 条超出记录限额的操作未保存参数；像素仍在版本快照中。", p.omitted_operations));
                }
                egui::ScrollArea::vertical().id_salt("ckpipe-versions").max_height(140.0).show(ui, |ui| {
                    for version in p.versions.iter().rev() {
                        ui.horizontal(|ui| {
                            let label = format!("{} · {} ← {}", version.id, version.label, version.parent.as_deref().unwrap_or("起点"));
                            if ui.selectable_label(app.coskit_ai.project_ui.base == version.id, label).clicked() {
                                app.coskit_ai.project_ui.base = version.id.clone();
                            }
                            if ui.small_button("恢复").clicked() {
                                action = Some(("coskit.project.restore", json!({"version":version.id})));
                            }
                        });
                    }
                });
                ui.collapsing("调整已记录的参数", |ui| {
                    ui.label("选择基准版本，再选一条可重放记录。通过控件调整参数；成功后生成分支版本。JSON 保留为高级入口。新建图层、外部文件与 AI 调用需重新执行。");
                    ui.label(format!("基准版本：{}", app.coskit_ai.project_ui.base));
                    egui::ScrollArea::vertical().id_salt("ckpipe-ops").max_height(100.0).show(ui, |ui| {
                        for op in p.operations.iter().rev().take(200) {
                            if ui.add_enabled(op.replayable, egui::Button::new(&op.command)).clicked() {
                                app.coskit_ai.project_ui.parameters = serde_json::to_string_pretty(
                                    &json!([{"command":op.command,"params":op.params,"active_layer":op.active_layer,"selected_layers":op.selected_layers}]),
                                )
                                .unwrap_or_default();
                            }
                        }
                    });
                    let mode = app.session.active().map(|s|s.doc.mode).unwrap_or(photocraft_doc::ColorMode::Rgb);
                    crate::pipeline_parameters::editor(ui, &mut app.coskit_ai.project_ui.parameters, mode);
                    ui.collapsing("高级 JSON", |ui| { ui.add(egui::TextEdit::multiline(&mut app.coskit_ai.project_ui.parameters).desired_rows(6).desired_width(f32::INFINITY).code_editor()); });
                    if ui.button("在基准版本重放并生成新版本").clicked() {
                        match serde_json::from_str::<serde_json::Value>(&app.coskit_ai.project_ui.parameters) {
                            Ok(steps) => action = Some(("coskit.project.replay", json!({"version":app.coskit_ai.project_ui.base,"steps":steps}))),
                            Err(e) => app.coskit_ai.project_ui.status = e.to_string(),
                        }
                    }
                });
            }
        });
        if let Some((command, params)) = action {
            app.coskit_ai.project_ui.status = match app.run(command, params) {
                Ok(v) if v.get("pending").and_then(serde_json::Value::as_bool)==Some(true) => "任务已开始，完成后生成版本；可在进度栏取消".into(),
                Ok(v) => format!("完成：{v}"),
                Err(e) => e,
            };
        }
        ui.label(&app.coskit_ai.project_ui.status);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_preview_does_not_decode_full_resolution_versions_and_cache_is_byte_bounded() {
        let mut session=photocraft_engine::Session::new();
        session.execute("file.new",json!({"width":1200,"height":800,"background":"white"})).unwrap();
        session.execute("coskit.project.checkpoint",json!({})).unwrap();
        let mut project=(**session.active().unwrap().pipeline.as_ref().unwrap()).clone();
        let hash=project.versions[0].snapshot.clone();
        assert!(project.previews.contains_key(&hash));
        project.objects.clear();project.snapshots.clear();
        let (metadata, buffer)=render_project_preview(&project,&hash).unwrap();
        assert!(metadata.layers.is_empty());
        assert_eq!(buffer.rect.width(),384);
        assert_eq!(session.active().unwrap().doc.size,photocraft_geom::Size::new(1200,800));
        let ctx=egui::Context::default();
        let mut state=State { budget_bytes:700_000,..Default::default() };
        for key in ["a","b"] {
            let texture=ctx.load_texture(key,egui::ColorImage::filled([400,400],egui::Color32::WHITE),Default::default());
            state.previews.insert(key.into(),Ok(texture));state.order.push_back(key.into());
        }
        trim_previews(&mut state);
        assert_eq!(state.cached_bytes,640_000);
        assert!(!state.previews.contains_key("a"));
        assert!(state.previews.contains_key("b"));
    }

    #[test]
    fn version_previews_use_snapshot_pixels_without_retaining_full_documents() {
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", json!({"width":64,"height":32,"background":"white"})).unwrap();
        let doc = session.active().unwrap().doc.clone();
        let bytes = photocraft_format::save_to_bytes(&doc, &Default::default()).unwrap();
        let (metadata, buffer) = render_preview(&bytes).unwrap();
        assert!(metadata.layers.is_empty());
        assert_eq!(metadata.size, doc.size);
        assert_eq!(buffer.rect.width(), 64);
        assert_eq!(buffer.rect.height(), 32);
        let pixels = buffer.to_rgba8();
        assert!(pixels.pixels.chunks_exact(4).all(|p| p == [255, 255, 255, 255]));
        assert_eq!(session.active().unwrap().doc, doc);
        assert!(render_preview(b"invalid snapshot").is_err());
    }

    #[test]
    fn changing_documents_clears_selected_version_and_replay_parameters() {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        let ctx = egui::Context::default();
        app.run("file.new", json!({"width":8,"height":8})).unwrap();
        app.run("coskit.project.checkpoint", json!({"label":"First"})).unwrap();
        synchronize(&mut app, &ctx);
        assert_eq!(app.coskit_ai.project_ui.base, "v1");
        app.coskit_ai.project_ui.parameters = "private old parameters".into();
        app.coskit_ai.project_ui.compare = true;
        app.run("file.new", json!({"width":16,"height":16})).unwrap();
        synchronize(&mut app, &ctx);
        assert!(app.coskit_ai.project_ui.parameters.is_empty());
        assert!(app.coskit_ai.project_ui.base.is_empty());
        assert!(!app.coskit_ai.project_ui.compare);
    }
    #[test]
    fn project_view_controls_validate_atomically_and_do_not_edit_pixels(){
        let mut app=PhotocraftApp::new(photocraft_engine::Session::new(),crate::Services::default());let ctx=egui::Context::default();
        app.run("file.new",json!({"width":64,"height":48})).unwrap();app.run("coskit.project.checkpoint",json!({})).unwrap();
        let before=app.session.active().unwrap().project_document();
        control(&mut app,&ctx,&json!({"version":"v1","compare":true,"zoom":2,"center":[24,20],"difference":true})).unwrap();
        let view=inspect(&app.coskit_ai.project_ui);
        for invalid in [json!({"compare":false,"zoom":-1}),json!({"version":"missing"}),json!({"center":[80,0]}),json!({"operation":9999}),json!({"unknown":true})] {
            assert!(control(&mut app,&ctx,&invalid).is_err());assert_eq!(inspect(&app.coskit_ai.project_ui),view);
        }
        assert_eq!(before,app.session.active().unwrap().project_document());
    }

}
