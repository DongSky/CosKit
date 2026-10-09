//! A native conversation panel sharing the editor's active document and undo stack.
use crate::PhotocraftApp;
use base64::Engine as _;
use photocraft_doc::{DocId, Document};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub struct Request {
    pub document: Arc<Document>,
    pub revision: u64,
    pub active_layer: Option<photocraft_doc::LayerId>,
    pub selected_layers: Vec<photocraft_doc::LayerId>,
    pub prompt: String,
    pub options: Value,
    pub references: Vec<(String, Vec<u8>)>,
    pub cancel: Arc<AtomicBool>,
}
pub enum Event {
    Progress(String),
    Done { png: Vec<u8>, note: String },
    Failed(String),
    Trace(Value),
    HarnessDone { pcraft: Vec<u8>, note: String },
    Unchanged(String),
}
pub type References = Vec<(String, Vec<u8>)>;
pub type PickReferences = Box<dyn Fn() -> Result<References, String>>;
pub type SaveHistory = Box<dyn Fn(&[Message]) -> Result<(), String>>;
pub struct Service {
    pub start: Box<dyn Fn(Request) -> mpsc::Receiver<Event>>,
    pub load_settings: Box<dyn Fn() -> Value>,
    pub save_settings: Box<dyn Fn(Value) -> Result<(), String>>,
    pub pick_references: PickReferences,
    pub load_history: Box<dyn Fn() -> Vec<Message>>,
    pub save_history: SaveHistory,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub document: String,
    pub role: String,
    pub text: String,
    #[serde(default)]
    pub conversation: u64,
}
pub struct Pending {
    pub document: DocId,
    pub revision: u64,
    pub name: String,
    pub png: Vec<u8>,
    pub pcraft: Option<Vec<u8>>,
}
pub struct State {
    pub open: bool,
    pub project_ui: crate::pipeline_ui::State,
    pub prompt: String,
    pub messages: Vec<Message>,
    pub status: String,
    outcome: &'static str,
    last_error: Option<String>,
    last_result: Value,
    trace: Vec<Value>,
    pub options: Value,
    pub references: Vec<(String, Vec<u8>)>,
    pub pending: Option<Pending>,
    pub receiver: Option<mpsc::Receiver<Event>>,
    source: Option<(DocId, u64, String, String)>,
    cancel: Arc<AtomicBool>,
    settings_open: bool,
    settings: Value,
    loaded: bool,
    conversations: std::collections::HashMap<DocId, u64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: true,
            project_ui: Default::default(),
            prompt: String::new(),
            messages: Vec::new(),
            status: String::new(),
            outcome: "idle",
            last_error: None,
            last_result: Value::Null,
            trace: Vec::new(),
            options: json!({"harness_enabled":true,"harness_max_steps":16,"harness_max_images":3,"harness_max_rollbacks":3,"retouch":true,"background":false,"effects":false,"agent_mode":true,"save_intermediates":true,"combined_mode":false,"review_enabled":false}),
            references: Vec::new(),
            pending: None,
            receiver: None,
            source: None,
            cancel: Arc::new(AtomicBool::new(false)),
            settings_open: false,
            settings: Value::Null,
            loaded: false,
            conversations: Default::default(),
        }
    }
}
fn hydrate_project(app: &mut PhotocraftApp) {
    let Some(st) = app.session.active() else {
        return;
    };
    if app.coskit_ai.conversations.contains_key(&st.doc.id) {
        return;
    }
    let conversation = app.coskit_ai.messages.iter().map(|m| m.conversation).max().unwrap_or(0).saturating_add(1);
    if let Some(p) = &st.pipeline {
        for value in &p.conversations {
            if let Ok(mut message) = serde_json::from_value::<Message>(value.clone()) {
                message.conversation = conversation;
                message.document = st.doc.name.clone();
                app.coskit_ai.messages.push(message);
            }
        }
    }
    app.coskit_ai.conversations.insert(st.doc.id, conversation);
}
fn archive_run(app: &mut PhotocraftApp, document: DocId) {
    if app.coskit_ai.trace.is_empty() {
        return;
    }
    let trace = json!({"events":app.coskit_ai.trace});
    if let Err(e) = app.session.execute("coskit.project.annotate", json!({"document":document.0,"run":trace})) {
        app.coskit_ai.status = e.to_string();
    }
}
fn persist(app: &mut PhotocraftApp) {
    for (document, conversation) in &app.coskit_ai.conversations {
        if !app.session.documents().iter().any(|s| s.doc.id == *document) {
            continue;
        }
        let messages: Vec<Value> =
            app.coskit_ai.messages.iter().filter(|m| m.conversation == *conversation).filter_map(|m| serde_json::to_value(m).ok()).collect();
        let previous = app
            .session
            .documents()
            .iter()
            .find(|s| s.doc.id == *document)
            .and_then(|s| s.pipeline.as_ref())
            .map(|p| p.conversations.as_slice())
            .unwrap_or_default();
        if previous != messages.as_slice()
            && let Err(e) = app.session.execute("coskit.project.annotate", json!({"document":document.0,"messages":messages}))
        {
            app.coskit_ai.status = e.to_string();
        }
    }

    if let Some(service) = &app.services.coskit_ai
        && let Err(e) = (service.save_history)(&app.coskit_ai.messages)
    {
        app.coskit_ai.status = format!("对话记录保存失败：{e}");
    }
}
pub fn start(app: &mut PhotocraftApp, prompt: String) -> Result<(), String> {
    hydrate_project(app);
    if app.coskit_ai.receiver.is_some() {
        return Err("已有 AI 编辑正在运行".into());
    }
    if app.coskit_ai.pending.is_some() {
        return Err("请先处理上一次 AI 结果".into());
    }
    if prompt.trim().is_empty() || prompt.len() > 32_000 {
        return Err("请输入 1–32000 字节的编辑指令".into());
    }
    let state = app.session.active().ok_or("请先打开图像或新建文档")?;
    let service = app.services.coskit_ai.as_ref().ok_or("AI 服务不可用")?;
    let source = (state.doc.id, state.revision, state.doc.name.clone(), prompt.clone());
    let next = app.coskit_ai.messages.iter().map(|m| m.conversation).max().unwrap_or(0).saturating_add(1);
    let conversation = *app.coskit_ai.conversations.entry(state.doc.id).or_insert(next);
    let cancel = Arc::new(AtomicBool::new(false));
    let context: Vec<String> = app
        .coskit_ai
        .messages
        .iter()
        .rev()
        .filter(|m| m.conversation == conversation)
        .take(12)
        .map(|m| format!("{}: {}", m.role, m.text.chars().take(500).collect::<String>()))
        .collect();
    let prompt_with_context = if context.is_empty() {
        prompt.clone()
    } else {
        format!("此前对话（仅作上下文，当前图片为本次编辑起点）：\n{}\n\n当前用户指令：{}", context.into_iter().rev().collect::<Vec<_>>().join("\n"), prompt)
    };
    let request = Request {
        document: Arc::new(state.project_document()),
        revision: state.revision,
        active_layer: state.active_layer,
        selected_layers: state.selected_layers(),
        prompt: prompt_with_context,
        options: app.coskit_ai.options.clone(),
        references: app.coskit_ai.references.clone(),
        cancel: cancel.clone(),
    };
    app.coskit_ai.receiver = Some((service.start)(request));
    app.coskit_ai.cancel = cancel;
    app.coskit_ai.messages.push(Message { document: source.2.clone(), role: "你".into(), text: prompt, conversation });
    app.coskit_ai.source = Some(source);
    app.coskit_ai.prompt.clear();
    app.coskit_ai.status = "正在准备当前文档的图像与选区…".into();
    app.coskit_ai.outcome = "running";
    app.coskit_ai.last_error = None;
    app.coskit_ai.last_result = Value::Null;
    app.coskit_ai.trace.clear();
    persist(app);
    Ok(())
}
pub fn poll(app: &mut PhotocraftApp) {
    if !app.coskit_ai.loaded {
        if let Some(service) = &app.services.coskit_ai {
            app.coskit_ai.messages = (service.load_history)();
        }
        app.coskit_ai.loaded = true;
    }
    hydrate_project(app);
    let events: Vec<_> = app.coskit_ai.receiver.as_ref().map(|r| r.try_iter().collect()).unwrap_or_default();
    for event in events {
        match event {
            Event::Progress(s) => app.coskit_ai.status = s,
            Event::Failed(e) => {
                app.coskit_ai.receiver = None;
                app.coskit_ai.outcome = if app.coskit_ai.cancel.load(Ordering::Relaxed) { "cancelled" } else { "failed" };
                app.coskit_ai.last_error = Some(e.clone());
                app.coskit_ai.status = format!("编辑失败：{e}");
                if let Some(source) = app.coskit_ai.source.take() {
                    archive_run(app, source.0);
                    let conversation = app.coskit_ai.conversations.get(&source.0).copied().unwrap_or(0);
                    app.coskit_ai.messages.push(Message { document: source.2, role: "错误".into(), text: e, conversation });
                }
                persist(app);
            }
            Event::Done { png, note } => receive_result(app, png, None, note),
            Event::HarnessDone { pcraft, note } => receive_result(app, Vec::new(), Some(pcraft), note),
            Event::Trace(value) => {
                if app.coskit_ai.trace.len() < 160 {
                    app.coskit_ai.trace.push(value);
                }
            }
            Event::Unchanged(note) => {
                app.coskit_ai.receiver = None;
                let cancelled = app.coskit_ai.cancel.load(Ordering::Relaxed);
                app.coskit_ai.outcome = if cancelled { "cancelled" } else { "unchanged" };
                app.coskit_ai.status = if cancelled { "编辑已取消".into() } else { note.clone() };
                if let Some(source) = app.coskit_ai.source.take() {
                    archive_run(app, source.0);
                    let conversation = app.coskit_ai.conversations.get(&source.0).copied().unwrap_or(0);
                    app.coskit_ai.messages.push(Message { document: source.2, role: "CosKit".into(), text: app.coskit_ai.status.clone(), conversation });
                }
                persist(app);
            }
        }
    }
}
fn receive_result(app: &mut PhotocraftApp, png: Vec<u8>, pcraft: Option<Vec<u8>>, note: String) {
    app.coskit_ai.receiver = None;
    if app.coskit_ai.cancel.load(Ordering::Relaxed) {
        app.coskit_ai.source = None;
        app.coskit_ai.status = "编辑已取消，结果未应用".into();
        app.coskit_ai.outcome = "cancelled";
        return;
    }
    if let Some((document, revision, name, prompt)) = app.coskit_ai.source.take() {
        archive_run(app, document);
        let conversation = app.coskit_ai.conversations.get(&document).copied().unwrap_or(0);
        app.coskit_ai.messages.push(Message { document: name, role: "CosKit".into(), text: note, conversation });
        app.coskit_ai.pending = Some(Pending { document, revision, name: format!("AI · {prompt}"), png, pcraft });
        if let Err(e) = apply_pending(app) {
            app.coskit_ai.outcome = "pending";
            app.coskit_ai.last_error = Some(e.clone());
            app.coskit_ai.status = e;
        }
    }
    persist(app);
}
fn apply_pending(app: &mut PhotocraftApp) -> Result<(), String> {
    let pending = app.coskit_ai.pending.as_ref().ok_or("没有待应用的结果")?;
    let (command, p) = if let Some(pcraft) = &pending.pcraft {
        ("coskit.harness.apply", json!({"document":pending.document.0,"revision":pending.revision,"pcraft":base64::prelude::BASE64_STANDARD.encode(pcraft)}))
    } else {
        (
            "coskit.ai.apply",
            json!({"document":pending.document.0,"revision":pending.revision,"name":pending.name,"selectionApplied":true,
            "png":base64::prelude::BASE64_STANDARD.encode(&pending.png)}),
        )
    };
    let result = app.session.execute(command, p).map_err(|e| e.to_string())?;
    app.coskit_ai.last_result = json!({"document":pending.document.0,"layer":result["layer"],"layers":result["layers"],"reviewed":result["reviewed"]});
    app.coskit_ai.outcome = "applied";
    app.coskit_ai.last_error = None;
    app.coskit_ai.pending = None;
    app.coskit_ai.status = "已应用编辑结果，可整体撤销或继续编辑。".into();
    Ok(())
}
pub fn inspect(app: &PhotocraftApp) -> Value {
    json!({"busy":app.coskit_ai.receiver.is_some(),"pending":app.coskit_ai.pending.is_some(),"status":app.coskit_ai.status,
        "messages":app.coskit_ai.messages.len(),"open":app.coskit_ai.open,"options":app.coskit_ai.options,
        "harness":app.coskit_ai.trace,"outcome":app.coskit_ai.outcome,"error":app.coskit_ai.last_error,"result":app.coskit_ai.last_result,
        "cancelRequested":app.coskit_ai.cancel.load(Ordering::Relaxed),
        "source":app.coskit_ai.source.as_ref().map(|s|json!({"document":s.0.0,"revision":s.1})),
        "activeDocument":app.session.active().map(|d|json!({"id":d.doc.id.0,"revision":d.revision,"width":d.doc.size.width,"height":d.doc.size.height}))})
}
/// Control protocol and UI use the same state and cancellation path.
pub fn configure(app: &mut PhotocraftApp, options: &Value) -> Result<Value, String> {
    if app.coskit_ai.receiver.is_some() {
        return Err("AI 编辑运行时不能修改流程".into());
    }
    let object = options.as_object().ok_or("options must be an object")?;
    let allowed = ["retouch", "background", "effects", "agent_mode", "combined_mode", "review_enabled", "save_intermediates", "harness_enabled"];
    for (k, v) in object {
        let valid = if allowed.contains(&k.as_str()) {
            v.is_boolean()
        } else {
            let range = match k.as_str() {
                "harness_max_steps" => 1..=24,
                "harness_max_images" => 0..=6,
                "harness_max_rollbacks" => 0..=6,
                _ => return Err(format!("invalid AI option: {k}")),
            };
            v.as_u64().is_some_and(|n| range.contains(&n))
        };
        if !valid {
            return Err(format!("invalid AI option: {k}"));
        }
    }
    for (k, v) in object {
        app.coskit_ai.options[k] = v.clone();
    }
    Ok(inspect(app))
}
pub fn cancel(app: &mut PhotocraftApp) -> Result<Value, String> {
    if app.coskit_ai.receiver.is_none() {
        return Err("没有运行中的 AI 编辑".into());
    }
    app.coskit_ai.cancel.store(true, Ordering::Relaxed);
    app.coskit_ai.status = "正在取消…".into();
    Ok(inspect(app))
}
pub fn recover(app: &mut PhotocraftApp, action: &str) -> Result<Value, String> {
    let pending = app.coskit_ai.pending.as_ref().ok_or("没有待处理的 AI 结果")?;
    match action {
        "apply" => apply_pending(app)?,
        "open" => {
            let document = if let Some(pcraft) = &pending.pcraft {
                photocraft_format::load_from_bytes(pcraft).map_err(|e| e.to_string())?
            } else {
                photocraft_io::import("CosKit AI result.png", &pending.png).map_err(|e| e.to_string())?.document
            };
            app.session.add_document(document, None);
            app.sync_views();
            app.coskit_ai.pending = None;
            app.coskit_ai.status = "结果已作为新文档打开".into();
            app.coskit_ai.outcome = "opened";
            app.coskit_ai.last_error = None;
        }
        "discard" => {
            app.coskit_ai.pending = None;
            app.coskit_ai.status = "已丢弃待处理的 AI 结果".into();
            app.coskit_ai.outcome = "discarded";
            app.coskit_ai.last_error = None;
        }
        _ => return Err("action must be apply, open or discard".into()),
    }
    Ok(inspect(app))
}
pub fn panel(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    if app.services.coskit_ai.is_none() {
        return;
    }
    if !app.coskit_ai.open {
        egui::Panel::right("coskit-ai-collapsed").exact_size(36.0).resizable(false).show(ui, |ui| {
            if ui.button("AI").on_hover_text("展开 CosKit 对话编辑").clicked() {
                app.coskit_ai.open = true;
            }
        });
        return;
    }
    if ui.ctx().content_rect().width() < 1100.0 {
        egui::Panel::bottom("coskit-ai-bottom").default_size(250.0).resizable(true).show(ui, |ui| scroll_body(app, ui));
    } else {
        egui::Panel::right("coskit-ai").default_size(300.0).size_range(260.0..=480.0).resizable(true).show(ui, |ui| scroll_body(app, ui));
    }
}
fn scroll_body(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let height = ui.available_height();
    egui::ScrollArea::vertical().id_salt("coskit-ai-body").auto_shrink([false, false]).show(ui, |ui| body(app, ui, height));
}
fn body(app: &mut PhotocraftApp, ui: &mut egui::Ui, height: f32) {
    ui.horizontal(|ui| {
        ui.strong("CosKit · AI 对话编辑");
        if ui.small_button("设置").clicked() {
            if let Some(service) = &app.services.coskit_ai {
                app.coskit_ai.settings = (service.load_settings)();
            }
            app.coskit_ai.settings_open = true;
        }
        if ui.small_button("收起").clicked() {
            app.coskit_ai.open = false;
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt("coskit-chat").stick_to_bottom(true).max_height((height - 200.0).max(65.0)).show(ui, |ui| {
        if app.coskit_ai.messages.is_empty() {
            ui.label("描述修图目标。智能修图会选择原生工具或图像模型，对比效果，必要时回退重试；通过检查后保留分层应用，可整体撤销。");
        }
        for message in app
            .coskit_ai
            .messages
            .iter()
            .filter(|m| app.session.active().and_then(|d| app.coskit_ai.conversations.get(&d.doc.id)).is_some_and(|id| m.conversation == *id))
        {
            ui.strong(format!("{} · {}", message.role, message.document));
            ui.add(egui::Label::new(&message.text).wrap());
            ui.add_space(8.0);
        }
    });
    ui.separator();
    crate::pipeline_ui::panel(app, ui);
    workflow_options(app, ui);
    ui.add(egui::TextEdit::multiline(&mut app.coskit_ai.prompt).desired_rows(3).desired_width(f32::INFINITY).hint_text("描述你想怎样修改当前图像…"));
    let busy = app.coskit_ai.receiver.is_some();
    ui.horizontal(|ui| {
        if ui.add_enabled(!busy && app.coskit_ai.pending.is_none() && app.session.active().is_some(), egui::Button::new("发送编辑")).clicked()
            && let Err(e) = start(app, app.coskit_ai.prompt.clone())
        {
            app.coskit_ai.status = e;
        }
        if busy {
            ui.spinner();
            if ui.button("取消").clicked() {
                let _ = cancel(app);
            }
        }
    });
    if !app.coskit_ai.status.is_empty() {
        ui.add(egui::Label::new(&app.coskit_ai.status).wrap());
    }
    let project = app.session.active().and_then(|s| s.pipeline.as_ref());
    let visible_trace = if app.coskit_ai.receiver.is_some() {
        app.coskit_ai.trace.as_slice()
    } else {
        project.and_then(|p| p.runs.last()).and_then(|r| r.get("events")).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
    };
    if !visible_trace.is_empty() {
        ui.collapsing("修图过程与检查记录", |ui| {
            for event in visible_trace {
                ui.label(format!("{} · {}", event["phase"].as_str().unwrap_or(""), event["summary"].as_str().unwrap_or("")));
            }
        });
    }
    if app.coskit_ai.pending.is_some() {
        ui.horizontal_wrapped(|ui| {
            if ui.button("重试应用").clicked()
                && let Err(e) = apply_pending(app)
            {
                app.coskit_ai.status = e;
            }
            if ui.button("作为新文档打开").clicked()
                && let Err(e) = recover(app, "open")
            {
                app.coskit_ai.status = e;
            }
        });
    }
}
fn workflow_options(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    ui.collapsing("编辑流程与参考图", |ui| {
        let mut harness = app.coskit_ai.options["harness_enabled"].as_bool().unwrap_or(true);
        if ui.add_enabled(app.coskit_ai.receiver.is_none(), egui::Checkbox::new(&mut harness, "自主修图 · 分析、执行、反思与回退")).changed() {
            app.coskit_ai.options["harness_enabled"] = json!(harness);
        }
        if !harness {
            for (key, label) in [
                ("agent_mode", "传统模式：智能规划"),
                ("retouch", "人物精修"),
                ("background", "背景替换"),
                ("effects", "添加特效"),
                ("combined_mode", "合并执行"),
                ("review_enabled", "结果审核"),
            ] {
                let mut value = app.coskit_ai.options[key].as_bool().unwrap_or(false);
                if ui.checkbox(&mut value, label).changed() {
                    app.coskit_ai.options[key] = json!(value);
                }
            }
        }
        ui.label("只需描述目标，模型自行分析步骤；自主模式每次修改后都会检查效果。");
        for (key, label, min, max) in
            [("harness_max_steps", "最多决策轮数", 1, 24), ("harness_max_images", "最多生成调用", 0, 6), ("harness_max_rollbacks", "最多回退次数", 0, 6)]
        {
            let mut n = app.coskit_ai.options[key].as_u64().unwrap_or(3) as u32;
            ui.horizontal(|ui| {
                ui.label(label);
                if ui.add_enabled(app.coskit_ai.receiver.is_none(), egui::DragValue::new(&mut n).range(min..=max)).changed() {
                    app.coskit_ai.options[key] = json!(n);
                }
            });
        }
        if ui.button("添加参考图…").clicked()
            && let Some(service) = &app.services.coskit_ai
        {
            match (service.pick_references)() {
                Ok(refs) => app.coskit_ai.references.extend(refs),
                Err(e) => app.coskit_ai.status = e,
            }
        }
        let mut remove = None;
        for (i, (name, _)) in app.coskit_ai.references.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(name);
                if ui.small_button("移除").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            app.coskit_ai.references.remove(i);
        }
    });
}

/// A fixed composer and one scrolling conversation. Project versions live below the canvas.
pub fn studio_body(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let busy = app.coskit_ai.receiver.is_some();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("一起完成这张作品").size(16.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("选项", |ui| {
                ui.set_width(290.0);
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    if ui.button("模型设置…").clicked() {
                        if let Some(service) = &app.services.coskit_ai {
                            app.coskit_ai.settings = (service.load_settings)();
                        }
                        app.coskit_ai.settings_open = true;
                        ui.close();
                    }
                    workflow_options(app, ui);
                });
            });
        });
    });
    ui.add_space(8.0);
    egui::Panel::bottom("coskit-composer")
        .exact_size(if app.coskit_ai.pending.is_some() { 204.0 } else { 158.0 })
        .frame(egui::Frame::NONE.fill(t.dock).inner_margin(egui::Margin { top: 10, ..Default::default() }))
        .show(ui, |ui| {
            if app.coskit_ai.pending.is_some() {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("重试应用").clicked()
                        && let Err(e) = apply_pending(app)
                    {
                        app.coskit_ai.status = e;
                    }
                    if ui.button("在新文档打开结果").clicked()
                        && let Err(e) = recover(app, "open")
                    {
                        app.coskit_ai.status = e;
                    }
                });
            }
            let response = ui.add_sized(
                [ui.available_width(), 76.0],
                egui::TextEdit::multiline(&mut app.coskit_ai.prompt).hint_text("描述想要的效果，例如：自然肤色，保留冷色氛围…"),
            );
            let send_key = response.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
            let can_send = !busy && app.coskit_ai.pending.is_none() && app.session.active().is_some() && !app.coskit_ai.prompt.trim().is_empty();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if busy {
                    ui.spinner();
                    if ui.button("停止").clicked() {
                        let _ = cancel(app);
                    }
                } else {
                    ui.label(egui::RichText::new("自主分析 · 效果检查").small().color(t.text_dim));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if (ui.add_enabled(can_send, egui::Button::new(egui::RichText::new("开始修图").color(t.primary_text)).fill(t.primary_bg)).clicked()
                        || send_key && can_send)
                        && let Err(e) = start(app, app.coskit_ai.prompt.clone())
                    {
                        app.coskit_ai.status = e;
                    }
                });
            });
            ui.label(egui::RichText::new(format!("Ctrl+Enter 发送  ·  {} 张参考图", app.coskit_ai.references.len())).small().color(t.text_faint));
        });
    egui::ScrollArea::vertical().id_salt("coskit-studio-chat").auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
        let conversation = app.session.active().and_then(|d| app.coskit_ai.conversations.get(&d.doc.id)).copied();
        let messages: Vec<_> = app.coskit_ai.messages.iter().filter(|m| Some(m.conversation) == conversation).collect();
        if messages.is_empty() {
            ui.add_space(24.0);
            ui.label(egui::RichText::new("从一个想法开始").size(21.0).strong());
            ui.add_space(8.0);
            ui.label(egui::RichText::new("告诉我你希望怎样改变照片。我会选择工具、检查效果，并保留可恢复的版本。").color(t.text_dim));
            ui.add_space(20.0);
            for (label, prompt) in [
                ("自然肤色", "让肤色更自然，保留妆容、皮肤质感和环境氛围。"),
                ("整理光影", "优化人物与环境的光影关系，保留服装和面部细节。"),
                ("清理画面", "清理画面中分散注意力的小杂物，保留主体与场景布局。"),
            ] {
                if ui.add_sized([ui.available_width(), 36.0], egui::Button::new(label)).clicked() {
                    app.coskit_ai.prompt = prompt.into();
                }
                ui.add_space(4.0);
            }
        }
        for message in messages {
            egui::Frame::NONE.fill(if message.role == "你" { t.field } else { t.card }).corner_radius(6).inner_margin(12).show(ui, |ui| {
                ui.set_width((ui.available_width() - 2.0).max(0.0));
                ui.label(egui::RichText::new(&message.role).small().strong().color(if message.role == "你" { t.text_dim } else { t.accent }));
                ui.add(egui::Label::new(&message.text).wrap());
            });
            ui.add_space(12.0);
        }
        let on_source = app.coskit_ai.source.as_ref().is_some_and(|s| app.session.active().is_some_and(|d| d.doc.id == s.0));
        if !app.coskit_ai.status.is_empty() && (busy || on_source) {
            ui.label(egui::RichText::new(if busy { "正在处理" } else { "任务状态" }).strong().color(t.accent));
            ui.add(egui::Label::new(&app.coskit_ai.status).wrap());
        }
        let project = app.session.active().and_then(|s| s.pipeline.as_ref());
        let trace = if busy && on_source {
            app.coskit_ai.trace.as_slice()
        } else {
            project.and_then(|p| p.runs.last()).and_then(|r| r.get("events")).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
        };
        if !trace.is_empty() {
            ui.add_space(8.0);
            ui.collapsing("查看修图过程与效果检查", |ui| {
                for event in trace {
                    ui.add(egui::Label::new(format!("{} · {}", event["phase"].as_str().unwrap_or(""), event["summary"].as_str().unwrap_or(""))).wrap());
                }
            });
        }
    });
}

pub fn settings_window(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if !app.coskit_ai.settings_open {
        return;
    }
    let mut open = true;
    egui::Window::new("CosKit 模型设置").open(&mut open).resizable(true).vscroll(true).default_height(600.0).default_width(460.0).show(ctx, |ui| {
        ui.label("留空时读取 .env 配置。密钥只用于模型请求并保存在本机设置中。");
        for (prefix, label) in [("text", "文本与规划模型"), ("image", "图像编辑模型"), ("review", "审核模型（可选）")] {
            ui.separator();
            ui.strong(label);
            for (field, title, password) in [
                ("provider", "供应商：openai / gemini / qwen", false),
                ("base_url", "API Base URL", false),
                ("model", "模型名称", false),
                ("api_key", "API Key", true),
            ] {
                let key = format!("{prefix}_{field}");
                let mut value = app.coskit_ai.settings[&key].as_str().unwrap_or_default().to_string();
                ui.label(title);
                if ui.add(egui::TextEdit::singleline(&mut value).password(password).desired_width(f32::INFINITY)).changed() {
                    app.coskit_ai.settings[&key] = json!(value);
                }
            }
        }
        if ui.add_enabled(app.coskit_ai.receiver.is_none(), egui::Button::new("保存模型设置")).clicked()
            && let Some(service) = &app.services.coskit_ai
        {
            match (service.save_settings)(app.coskit_ai.settings.clone()) {
                Ok(()) => {
                    app.coskit_ai.settings_open = false;
                    app.coskit_ai.status = "模型设置已保存".into();
                }
                Err(e) => app.coskit_ai.status = e,
            }
        }
    });
    if !open {
        app.coskit_ai.settings_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> PhotocraftApp {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.session.execute("file.new", json!({"width":8,"height":8,"background":"white"})).unwrap();
        app.services.coskit_ai = Some(Service {
            start: Box::new(|request| {
                let (tx, rx) = mpsc::channel();
                let (png, _) = photocraft_engine::coskit_ai::snapshot(&request.document).unwrap();
                tx.send(Event::Done { png, note: "synthetic response".into() }).unwrap();
                rx
            }),
            load_settings: Box::new(|| json!({})),
            save_settings: Box::new(|_| Ok(())),
            pick_references: Box::new(|| Ok(vec![])),
            load_history: Box::new(Vec::new),
            save_history: Box::new(|_| Ok(())),
        });
        poll(&mut app);
        app
    }
    #[test]
    fn ckpipe_conversation_reopens_and_is_isolated_from_another_document() {
        let mut app = app();
        start(&mut app, "keep texture".into()).unwrap();
        poll(&mut app);
        let bytes = photocraft_format::pipeline::save(&app.session.active().unwrap().project_document(), &Default::default()).unwrap();
        let doc = photocraft_format::load_from_bytes(&bytes).unwrap();
        app.session.add_document(doc, None);
        poll(&mut app);
        let id = app.session.active().unwrap().doc.id;
        let c = app.coskit_ai.conversations[&id];
        let messages: Vec<_> = app.coskit_ai.messages.iter().filter(|m| m.conversation == c).collect();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "keep texture");
        app.session.execute("file.new", json!({"width":3,"height":3})).unwrap();
        poll(&mut app);
        let c2 = app.coskit_ai.conversations[&app.session.active().unwrap().doc.id];
        assert_ne!(c, c2);
        assert!(!app.coskit_ai.messages.iter().any(|m| m.conversation == c2));
    }
    #[test]
    fn cancelling_a_queued_success_never_applies_the_result() {
        let mut app = app();
        let before = app.session.active().unwrap().doc.clone();
        start(&mut app, "cancel this queued response".into()).unwrap();
        cancel(&mut app).unwrap();
        poll(&mut app);
        assert_eq!(app.session.active().unwrap().doc.layers, before.layers);
        assert!(app.coskit_ai.pending.is_none());
        assert!(app.coskit_ai.receiver.is_none());
    }
    #[test]
    fn options_validate_atomically_and_pending_can_be_recovered() {
        let mut app = app();
        let before = app.coskit_ai.options.clone();
        assert!(configure(&mut app, &json!({"background":true,"api_key":"forbidden"})).is_err());
        assert_eq!(before, app.coskit_ai.options);
        configure(&mut app, &json!({"background":true})).unwrap();
        start(&mut app, "edit".into()).unwrap();
        assert!(configure(&mut app, &json!({"effects":true})).is_err());
        app.session.execute("layer.new.layer", json!({"name":"concurrent edit"})).unwrap();
        poll(&mut app);
        assert!(recover(&mut app, "apply").is_err());
        recover(&mut app, "open").unwrap();
        assert_eq!(app.session.documents().len(), 2);
        assert_eq!(app.ui.views.len(), 2, "recovered result must have a view before the same frame renders the status bar");
        assert!(app.coskit_ai.pending.is_none());
    }
    #[test]
    fn response_adds_one_layer_in_native_history() {
        let mut app = app();
        let count = app.session.active().unwrap().doc.layers.len();
        start(&mut app, "Synthetic edit".into()).unwrap();
        poll(&mut app);
        assert_eq!(app.session.active().unwrap().doc.layers.len(), count + 1);
        assert!(app.coskit_ai.receiver.is_none());
        assert!(app.coskit_ai.pending.is_none());
        app.session.execute("edit.undo", json!({})).unwrap();
        assert_eq!(app.session.active().unwrap().doc.layers.len(), count);
    }
    #[test]
    fn concurrent_local_edit_retains_ai_result_without_overwriting() {
        let mut app = app();
        start(&mut app, "Synthetic edit".into()).unwrap();
        app.session.execute("layer.new.layer", json!({"name":"Keep my edit"})).unwrap();
        let doc = app.session.active().unwrap().doc.clone();
        poll(&mut app);
        assert_eq!(app.session.active().unwrap().doc.layers, doc.layers);
        assert!(app.coskit_ai.pending.is_some());
    }
    #[test]
    fn same_named_documents_do_not_share_prompt_context() {
        let mut app = app();
        start(&mut app, "first document only".into()).unwrap();
        poll(&mut app);
        app.session.execute("file.new", json!({"width":8,"height":8,"background":"white"})).unwrap();
        start(&mut app, "second document".into()).unwrap();
        poll(&mut app);
        assert_ne!(app.coskit_ai.messages[0].conversation, app.coskit_ai.messages[2].conversation);
    }
    #[test]
    fn no_document_and_empty_prompt_fail_without_starting() {
        let mut app = app();
        assert!(start(&mut app, " ".into()).is_err());
        app.session.close(0);
        assert!(start(&mut app, "edit".into()).is_err());
        assert!(app.coskit_ai.receiver.is_none());
    }
    #[test]
    fn harness_native_result_is_pending_when_source_changes_and_opens_as_layers() {
        let mut app = app();
        start(&mut app, "test".into()).unwrap();
        let source = app.session.active().unwrap().doc.clone();
        let mut native = (*source).clone();
        native.layers.push(photocraft_doc::Layer::new("harness", photocraft_doc::LayerContent::Raster(photocraft_raster::Surface::new(source.pixel_format()))));
        let bytes = photocraft_format::save_to_bytes(&native, &Default::default()).unwrap();
        app.session.execute("layer.new.layer", json!({"name":"user simultaneous change"})).unwrap();
        receive_result(&mut app, vec![], Some(bytes), "reviewed".into());
        assert_eq!(inspect(&app)["outcome"], "pending");
        let user = app.session.active().unwrap().doc.clone();
        recover(&mut app, "open").unwrap();
        assert_eq!(app.session.documents().len(), 2);
        assert_eq!(app.session.documents()[0].doc, user);
        assert_eq!(app.session.active().unwrap().doc.layers.len(), 2);
    }
    #[test]
    fn harness_options_are_atomic_and_native_cancel_drops_queued_result() {
        let mut app = app();
        let before = app.coskit_ai.options.clone();
        assert!(configure(&mut app, &json!({"harness_enabled":false,"harness_max_steps":100})).is_err());
        assert_eq!(app.coskit_ai.options, before);
        configure(&mut app, &json!({"harness_max_images":0,"harness_max_steps":4})).unwrap();
        start(&mut app, "test".into()).unwrap();
        let original = app.session.active().unwrap().doc.clone();
        cancel(&mut app).unwrap();
        receive_result(&mut app, vec![], Some(vec![]), "reviewed".into());
        assert_eq!(app.session.active().unwrap().doc, original);
        assert!(app.coskit_ai.pending.is_none());
    }
}
