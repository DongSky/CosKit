//! Read-only, synchronised full-resolution version comparison with one bounded region worker.
use crate::{PhotocraftApp, theme::Tokens};
use egui::{Color32, Sense, vec2};
use photocraft_doc::{DocId, Document};
use photocraft_geom::Rect;
use std::sync::{Arc, mpsc};

#[derive(Clone, PartialEq)]
struct Key {
    document: DocId,
    revision: u64,
    hash: String,
    rect: Rect,
    width: u32,
    height: u32,
    monitor: u64,
}
struct Rendered {
    version: Arc<Document>,
    before: photocraft_compose::Buffer,
    after: photocraft_compose::Buffer,
}
struct Worker {
    key: Key,
    rx: mpsc::Receiver<Result<Rendered, String>>,
}
#[derive(Default)]
pub(super) struct State {
    pub zoom: f32,
    pub center: Option<[f32; 2]>,
    pub difference: bool,
    source: Option<(DocId, String, Arc<Document>)>,
    worker: Option<Worker>,
    shown: Option<Key>,
    textures: Vec<egui::TextureHandle>,
    error: Option<String>,
}
impl State {
    pub fn reset(&mut self) {
        let worker = self.worker.take();
        *self = Self::default();
        self.worker = worker;
    }
    pub fn reap_closed(&mut self) {
        if self.worker.as_ref().is_some_and(|w| !matches!(w.rx.try_recv(), Err(mpsc::TryRecvError::Empty))) {
            self.worker = None;
        }
    }

    pub fn inspect(&self) -> serde_json::Value {
        serde_json::json!({"zoom":self.zoom,"center":self.center,"difference":self.difference,"loading":self.worker.is_some(),"textureBytes":self.textures.iter().map(|t|t.size()[0]*t.size()[1]*4).sum::<usize>(),"error":self.error})
    }
}
fn viewport(size: photocraft_geom::Size, panel: egui::Vec2, zoom: f32, center: Option<[f32; 2]>) -> (Rect, u32, u32, f32) {
    let fit = (panel.x / size.width.max(1) as f32).min(panel.y / size.height.max(1) as f32);
    let scale = if zoom <= 0.0 { fit } else { zoom.clamp(0.01, 16.0) };
    let width = (panel.x / scale).ceil().max(1.0).min(size.width as f32) as u32;
    let height = (panel.y / scale).ceil().max(1.0).min(size.height as f32) as u32;
    let center = center.unwrap_or([size.width as f32 / 2.0, size.height as f32 / 2.0]);
    let x = (center[0] - width as f32 / 2.0).round().clamp(0.0, size.width.saturating_sub(width) as f32) as i32;
    let y = (center[1] - height as f32 / 2.0).round().clamp(0.0, size.height.saturating_sub(height) as f32) as i32;
    (
        Rect::from_xywh(x, y, width, height),
        ((width as f32 * scale).round() as u32).clamp(1, 1024),
        ((height as f32 * scale).round() as u32).clamp(1, 768),
        scale,
    )
}
fn render(project: &photocraft_doc::pipeline::Pipeline, current: &Document, cached: Option<Arc<Document>>, key: &Key) -> Result<Rendered, String> {
    let version = match cached {
        Some(d) => d,
        None => Arc::new(photocraft_format::pipeline::load_snapshot(project, &key.hash).map_err(|e| e.to_string())?),
    };
    let before = photocraft_compose::render_reduced_rect(&version, key.rect, key.width, key.height);
    let after = photocraft_compose::render_reduced_rect(current, key.rect, key.width, key.height);
    Ok(Rendered { version, before, after })
}
pub(super) fn window(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if !app.coskit_ai.project_ui.compare {
        return;
    }
    let Some(st) = app.session.active() else { return };
    let Some(project) = st.pipeline.clone() else { return };
    let Some(version) = project.versions.iter().find(|v| v.id == app.coskit_ai.project_ui.base) else { return };
    let current = st.doc.clone();
    let revision = st.revision;
    let hash = version.snapshot.clone();
    let title = format!("{} · {}", version.id, version.label);
    let monitor = app.session.color.monitor().content_hash();
    let color = app.session.color.canvas_display(&current);
    let state = &mut app.coskit_ai.project_ui.comparison;
    let mut open = true;
    egui::Window::new("版本对比").open(&mut open).default_width(980.0).resizable(true).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button("适合窗口").clicked() {
                state.zoom = 0.0;
                state.center = None;
            }
            if ui.button("100% 原尺寸").clicked() {
                state.zoom = 1.0;
            }
            if ui.button("200% 放大镜").clicked() {
                state.zoom = 2.0;
            }
            ui.checkbox(&mut state.difference, "差异图 ×4");
        });
        ui.weak("左右同步缩放与拖动；滚轮放大局部。对比只读，不改变画布或图层。");
        let panel = vec2(((ui.available_width() - 12.0) / 2.0).clamp(80.0, 1024.0), (ctx.content_rect().height() - 280.0).clamp(100.0, 650.0));
        let (rect, width, height, scale) = viewport(current.size, panel, state.zoom, state.center);
        let desired = Key { document: current.id, revision, hash: hash.clone(), rect, width, height, monitor };
        let done = state.worker.as_ref().and_then(|w| match w.rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("对比任务已停止".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = done
            && let Some(worker) = state.worker.take()
            && worker.key == desired
        {
            state.shown = Some(desired.clone());
            state.textures.clear();
            state.error = None;
            let result = result.and_then(|r| {
                let old = app.session.color.canvas_display(&r.version).map_err(|e| e.to_string())?.to_rgba8(&r.before);
                let new = color.as_ref().map_err(|e| e.to_string())?.to_rgba8(&r.after);
                let mut difference = Vec::with_capacity(new.pixels.len());
                for (a, b) in old.pixels.chunks_exact(4).zip(new.pixels.chunks_exact(4)) {
                    // Compare displayed colours including alpha, rather than treating different profiles as equivalent.
                    for (x, y) in a.iter().take(3).zip(b.iter().take(3)) {
                        difference.push(x.abs_diff(*y).max(a[3].abs_diff(b[3])).saturating_mul(4));
                    }
                    difference.push(255);
                }
                for (name, pixels) in [("before", old.pixels), ("after", new.pixels), ("difference", difference)] {
                    state.textures.push(ctx.load_texture(
                        format!("ckpipe-compare-{name}"),
                        egui::ColorImage::from_rgba_unmultiplied([new.width as usize, new.height as usize], &pixels),
                        egui::TextureOptions::NEAREST,
                    ));
                }
                state.source = Some((current.id, hash.clone(), r.version));
                Ok(())
            });
            if let Err(e) = result {
                state.error = Some(e);
            }
        }
        if state.worker.is_none() && state.shown.as_ref() != Some(&desired) {
            let cached = state.source.as_ref().filter(|(id, h, _)| *id == current.id && *h == hash).map(|(_, _, d)| d.clone());
            let (tx, rx) = mpsc::channel();
            let key = desired.clone();
            let document = current.clone();
            let project = project.clone();
            let context = ctx.clone();
            state.worker = Some(Worker { key: desired.clone(), rx });
            let work = move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render(&project, &document, cached, &key)))
                    .unwrap_or_else(|_| Err("此版本无法生成对比；原工程未修改".into()));
                let _ = tx.send(result);
                context.request_repaint();
            };
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = std::thread::Builder::new().name("ckpipe-compare".into()).spawn(work);
            }
            #[cfg(target_arch = "wasm32")]
            work();
        }
        ui.label(format!("{:.0}% · 原图区域 ({}, {}) {} × {}", scale * 100.0, rect.x0, rect.y0, rect.width(), rect.height()));
        ui.columns(2, |cols| {
            for (index, col) in cols.iter_mut().enumerate() {
                col.label(if index == 0 {
                    title.as_str()
                } else if state.difference {
                    "当前 − 所选版本（显示差异 ×4）"
                } else {
                    "当前画面"
                });
                let (area, response) = col.allocate_exact_size(panel, Sense::drag());
                let target = egui::Rect::from_center_size(area.center(), vec2(width as f32, height as f32));
                crate::widgets::checker(col.painter(), target, 8.0);
                let texture_index = if index == 1 && state.difference { 2 } else { index };
                if state.shown.as_ref() == Some(&desired)
                    && let Some(texture) = state.textures.get(texture_index)
                {
                    col.painter().image(texture.id(), target, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                } else {
                    col.painter().text(
                        area.center(),
                        egui::Align2::CENTER_CENTER,
                        "准备原尺寸对比…",
                        egui::FontId::proportional(14.0),
                        Tokens::get(ctx).text_dim,
                    );
                }
                if response.dragged() {
                    let delta = col.input(|i| i.pointer.delta());
                    let c = state.center.unwrap_or([current.size.width as f32 / 2.0, current.size.height as f32 / 2.0]);
                    state.center =
                        Some([(c[0] - delta.x / scale).clamp(0.0, current.size.width as f32), (c[1] - delta.y / scale).clamp(0.0, current.size.height as f32)]);
                }
                if response.hovered() {
                    let scroll = col.input(|i| i.smooth_scroll_delta.y);
                    if scroll.abs() > 0.1 {
                        if let Some(pointer) = response.hover_pos() {
                            state.center = Some([rect.x0 as f32 + (pointer.x - target.min.x) / scale, rect.y0 as f32 + (pointer.y - target.min.y) / scale]);
                        }
                        state.zoom = (scale * (scroll * 0.004).exp()).clamp(0.01, 16.0);
                    }
                }
            }
        });
        if let Some(error) = &state.error {
            ui.colored_label(Tokens::get(ctx).text_dim, error);
        }
        if state.worker.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        }
    });
    app.coskit_ai.project_ui.compare = open;
    if !open {
        app.coskit_ai.project_ui.comparison.reset();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_to_one_roi_is_bounded_and_pan_clamps_without_resizing_document() {
        let size = photocraft_geom::Size::new(5472, 3648);
        let (r, w, h, scale) = viewport(size, vec2(450.0, 500.0), 1.0, Some([5472.0, 3648.0]));
        assert_eq!((r.width(), r.height(), w, h), (450, 500, 450, 500));
        assert_eq!(scale, 1.0);
        assert_eq!((r.x1, r.y1), (5472, 3648));
        let (_, w, h, _) = viewport(size, vec2(450.0, 500.0), 0.0, None);
        assert!(w <= 450 && h <= 500);
    }
    #[test]
    fn zoomed_comparison_reads_real_version_pixels_and_never_resamples_document() {
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", serde_json::json!({"width":128,"height":96,"background":"white","depth":16})).unwrap();
        session.execute("coskit.project.checkpoint", serde_json::json!({})).unwrap();
        session.execute("image.adjustments.invert", serde_json::json!({})).unwrap();
        let st = session.active().unwrap();
        let source = st.doc.clone();
        let project = st.pipeline.as_ref().unwrap();
        let key = Key {
            document: source.id,
            revision: st.revision,
            hash: project.versions[0].snapshot.clone(),
            rect: Rect::from_xywh(20, 20, 32, 24),
            width: 64,
            height: 48,
            monitor: 0,
        };
        let result = render(project, &source, None, &key).unwrap();
        assert_eq!((result.before.rect.width(), result.before.rect.height()), (32, 24));
        assert!(result.before.px.iter().all(|p| p[0] > 0.99));
        assert!(result.after.px.iter().all(|p| p[0] < 0.01));
        assert_eq!(source, session.active().unwrap().doc);
    }
}
