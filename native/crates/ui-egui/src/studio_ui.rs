//! CosKit workspace: one composable inspector, a quiet canvas and contextual tools.
//! Layout is view state; document actions always dispatch the existing command registry.
use crate::{PhotocraftApp, dock::Group, icons, theme::Tokens};
use egui::{Align, RichText, Sense, vec2};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Inspector {
    #[default]
    Ai,
    Adjust,
    Layers,
    Color,
    History,
    Navigator,
    Character,
    Parameters,
}
impl Inspector {
    fn label(self) -> &'static str {
        match self {
            Self::Ai => "AI 修图",
            Self::Adjust => "调整",
            Self::Layers => "图层",
            Self::Color => "颜色",
            Self::History => "历史",
            Self::Navigator => "导航",
            Self::Character => "文字",
            Self::Parameters => "编辑参数",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StudioState {
    pub enabled: bool,
    pub initialized: bool,
    pub inspector: Inspector,
    pub pinned: Option<Inspector>,
    pub versions: bool,
}
impl Default for StudioState {
    fn default() -> Self {
        Self { enabled: true, initialized: false, inspector: Inspector::Ai, pinned: None, versions: true }
    }
}
pub fn active(app: &PhotocraftApp) -> bool {
    app.services.coskit_ai.is_some() && app.ui.studio.enabled
}
pub fn initialize(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if app.services.coskit_ai.is_some() && !app.ui.studio.initialized {
        app.ui.studio.initialized = true;
        if app.ui.theme == crate::theme::ThemeKind::ProMedium {
            app.set_theme(ctx, crate::theme::ThemeKind::Studio);
        }
        crate::dock::persist(app, ctx);
    }
}

fn invoke(app: &mut PhotocraftApp, ui: &egui::Ui, id: &str) {
    if let Err(error) = crate::menus::invoke(app, ui.ctx(), id, json!({})) {
        app.ui.status = error;
        app.ui.status_error = true;
    }
}

pub fn header(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let custom = app.custom_titlebar;
    let bar = egui::Panel::top("coskit-header")
        .exact_size(52.0)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 16, right: 12, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let mut content_rect = ui.max_rect();
            if custom {
                content_rect.max.x -= crate::titlebar::WIDTH;
            }
            let mut content = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
            content.set_clip_rect(content_rect);
            let ui = &mut content;
            ui.horizontal_centered(|ui| {
                let (mark, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
                crate::brand::paint_mark(ui, mark);
                ui.label(RichText::new("CosKit").size(17.0).strong());
                ui.add_space(8.0);
                ui.menu_button("菜单", |ui| {
                    crate::menus::menu_bar(app, ui);
                });
                if icons::button(ui, "search", 30.0, app.ui.palette_open, "搜索全部命令 · Ctrl+K").clicked() {
                    app.ui.palette_open = !app.ui.palette_open;
                }
                let full = ui.available_rect_before_wrap();
                let compact = full.width() < 740.0;
                let right_width = if compact { 230.0 } else { 400.0 };
                let name_width = (full.width() - right_width - 12.0).max(20.0);
                let (name_rect, response) = ui.allocate_exact_size(vec2(name_width, 36.0), Sense::click_and_drag());
                if response.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if response.double_clicked() {
                    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                let title = app
                    .session
                    .active()
                    .map(|d| format!("{}  {}", d.doc.name, if d.is_dirty() { "· 未保存" } else { "· 已保存" }))
                    .unwrap_or_else(|| "开始新的创作".into());
                let mut job = egui::text::LayoutJob::simple_singleline(title, crate::theme::medium(13.0), t.text_dim);
                job.wrap = egui::text::TextWrapping::truncate_at_width(name_width);
                let galley = ui.painter().layout_job(job);
                ui.painter().galley(egui::pos2(name_rect.left(), name_rect.center().y - galley.size().y / 2.0), galley, t.text_dim);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(app.session.active().is_some(), egui::Button::new(RichText::new("导出").color(t.primary_text)).fill(t.primary_bg))
                        .clicked()
                    {
                        invoke(app, ui, "file.export.exportAs");
                    }
                    if ui.button("保存").on_hover_text("保存工程 · Ctrl+S").clicked() {
                        invoke(app, ui, "file.save");
                    }
                    if icons::button(ui, "redo-2", 30.0, false, "重做").clicked() {
                        invoke(app, ui, "edit.redo");
                    }
                    if icons::button(ui, "undo-2", 30.0, false, "撤销").clicked() {
                        invoke(app, ui, "edit.undo");
                    }
                    ui.menu_button(if compact { "布局" } else { "工作空间" }, |ui| {
                        for (label, tab, pin) in [
                            ("修图", Inspector::Adjust, Some(Inspector::Layers)),
                            ("绘制", Inspector::Color, Some(Inspector::Layers)),
                            ("AI 创作", Inspector::Ai, None),
                        ] {
                            if ui.button(label).clicked() {
                                app.ui.studio.inspector = tab;
                                app.ui.studio.pinned = pin;
                                app.ui.panels.dock = true;
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.checkbox(&mut app.ui.panels.dock, "显示侧栏");
                        ui.checkbox(&mut app.ui.studio.versions, "显示版本条");
                        if ui.button("完整面板布局").clicked() {
                            app.ui.studio.enabled = false;
                            ui.close();
                        }
                    });
                    if !compact && ui.selectable_label(app.ui.studio.versions, "版本").clicked() {
                        app.ui.studio.versions = !app.ui.studio.versions;
                    }
                });
            });
        });
    if custom {
        crate::titlebar::caption_buttons(app, ui, bar.response.rect);
    }
}

/// Keep a usable canvas; a second inspector stacks vertically and never steals another column.
pub fn sidebar(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let max = (ui.available_width() - 240.0).clamp(280.0, 460.0);
    egui::Panel::right("coskit-inspector")
        .default_size(360.0)
        .size_range(280.0..=max)
        .resizable(true)
        .frame(egui::Frame::NONE.fill(t.dock).inner_margin(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in [Inspector::Ai, Inspector::Adjust, Inspector::Layers] {
                    if ui.selectable_label(app.ui.studio.inspector == tab, tab.label()).clicked() {
                        app.ui.studio.inspector = tab;
                    }
                }
                ui.menu_button("更多", |ui| {
                    for tab in [Inspector::Color, Inspector::History, Inspector::Navigator, Inspector::Character, Inspector::Parameters] {
                        if ui.selectable_label(app.ui.studio.inspector == tab, tab.label()).clicked() {
                            app.ui.studio.inspector = tab;
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("固定当前面板").clicked() {
                        let tab = app.ui.studio.inspector;
                        app.ui.studio.pinned = Some(tab);
                        app.ui.studio.inspector = if tab == Inspector::Ai { Inspector::Layers } else { Inspector::Ai };
                        ui.close();
                    }
                    if app.ui.studio.pinned.is_some() && ui.button("取消固定").clicked() {
                        app.ui.studio.pinned = None;
                        ui.close();
                    }
                });
            });
            ui.add_space(12.0);
            if let Some(pin) = app.ui.studio.pinned.filter(|p| *p != app.ui.studio.inspector) {
                let maximum = (ui.available_height() - 160.0).max(100.0);
                let minimum: f32 = if pin == Inspector::Ai { 250.0 } else { 180.0 };
                let height = (ui.available_height() * 0.40).clamp(minimum.min(maximum), maximum);
                egui::Panel::bottom("coskit-pinned")
                    .default_size(height)
                    .size_range(100.0..=maximum)
                    .resizable(true)
                    .frame(egui::Frame::NONE.fill(t.dock))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.strong(format!("{} · 已固定", pin.label()));
                            if ui.small_button("取消固定").clicked() {
                                app.ui.studio.pinned = None;
                            }
                        });
                        ui.add_space(8.0);
                        inspector(app, ui, pin, "pinned");
                    });
            }
            inspector(app, ui, app.ui.studio.inspector, "main");
        });
}

fn inspector(app: &mut PhotocraftApp, ui: &mut egui::Ui, tab: Inspector, id: &str) {
    ui.push_id(id, |ui| {
        if tab == Inspector::Ai {
            crate::coskit_ai::studio_body(app, ui);
            return;
        }
        egui::ScrollArea::vertical().id_salt(("inspector", id, tab.label())).auto_shrink([false, false]).show(ui, |ui| match tab {
            Inspector::Adjust => {
                ui.strong("光线与色彩");
                ui.add_space(8.0);
                crate::panels::dock_body(app, ui, Group::Properties, 1);
                ui.add_space(16.0);
                ui.strong("当前图层属性");
                ui.add_space(8.0);
                crate::panels::dock_body(app, ui, Group::Properties, 0);
            }
            Inspector::Parameters => crate::pipeline_ui::panel(app, ui),
            Inspector::Ai => {}
            _ => {
                let (group, labels): (Group, &[&str]) = match tab {
                    Inspector::Layers => (Group::Layers, &["图层", "通道", "路径"]),
                    Inspector::Color if Tokens::get(ui.ctx()).pro => (Group::Color, &["颜色", "色板", "渐变", "图案"]),
                    Inspector::Color => (Group::Color, &["色板", "颜色", "渐变", "图案"]),
                    Inspector::History => (Group::History, &["历史", "动作", "图层复合"]),
                    Inspector::Navigator => (Group::Navigator, &["导航", "直方图", "信息"]),
                    _ => (Group::Character, &["字符", "段落"]),
                };
                let mut selected = *dock_tab(app, group);
                ui.horizontal(|ui| {
                    for (index, label) in labels.iter().enumerate() {
                        ui.selectable_value(&mut selected, index, *label);
                    }
                });
                *dock_tab(app, group) = selected;
                ui.add_space(12.0);
                crate::panels::dock_body(app, ui, group, selected);
            }
        });
    });
}

fn dock_tab(app: &mut PhotocraftApp, group: Group) -> &mut usize {
    let tabs = &mut app.ui.dock_tabs;
    match group {
        Group::Color => &mut tabs.color,
        Group::Layers => &mut tabs.layers,
        Group::History => &mut tabs.history,
        Group::Navigator => &mut tabs.navigator,
        Group::Character => &mut tabs.character,
        Group::Properties => &mut tabs.properties,
    }
}

pub fn follow_panel(app: &mut PhotocraftApp, id: &str) {
    if !active(app) {
        return;
    }
    let p = &app.ui.panels;
    let target = match id {
        "window.panel.info" | "window.panel.histogram" | "window.panel.navigator" if p.navigator => Some(Inspector::Navigator),
        "window.panel.actions" | "window.panel.history" | "window.panel.layerComps" if p.history => Some(Inspector::History),
        "window.panel.channels" | "window.panel.paths" | "window.panel.layers" if p.layers => Some(Inspector::Layers),
        "window.panel.properties" | "window.panel.adjustments" if p.properties => Some(Inspector::Adjust),
        "window.panel.color" | "window.panel.swatches" | "window.panel.gradients" | "window.panel.patterns" if p.color => Some(Inspector::Color),
        "window.panel.character" | "window.panel.paragraph" | "type.panels.character" | "type.panels.paragraph" if p.character => Some(Inspector::Character),
        _ => None,
    };
    if let Some(tab) = target {
        app.ui.studio.inspector = tab;
        app.ui.panels.dock = true;
    }
}

/// Existing Window commands still expose all panels in the new workspace.
pub fn reveal_panel(app: &mut PhotocraftApp, id: &str) -> bool {
    if !active(app) {
        return false;
    }
    let tab = match id {
        "window.toggle.layers" => Inspector::Layers,
        "window.toggle.properties" => Inspector::Adjust,
        "window.toggle.color" => Inspector::Color,
        "window.toggle.history" => Inspector::History,
        "window.toggle.navigator" => Inspector::Navigator,
        _ => return false,
    };
    app.ui.panels.dock = true;
    app.ui.studio.inspector = tab;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> PhotocraftApp {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.services.coskit_ai = Some(crate::coskit_ai::Service {
            start: Box::new(|_| std::sync::mpsc::channel().1),
            load_settings: Box::new(|| json!({})),
            save_settings: Box::new(|_| Ok(())),
            pick_references: Box::new(|| Ok(vec![])),
            load_history: Box::new(Vec::new),
            save_history: Box::new(|_| Ok(())),
        });
        app
    }

    #[test]
    fn workspace_preferences_survive_restart_and_legacy_switch() {
        let mut app = app();
        let ctx = egui::Context::default();
        initialize(&mut app, &ctx);
        assert_eq!(app.ui.theme, crate::theme::ThemeKind::Studio);
        app.ui.studio.inspector = Inspector::Color;
        app.ui.studio.pinned = Some(Inspector::Layers);
        app.ui.studio.versions = false;
        crate::dock::persist(&mut app, &ctx);
        let saved = app.session.prefs().panel_layout.clone();
        let mut next = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        crate::dock::apply(&mut next, &saved);
        assert_eq!(next.ui.studio, app.ui.studio);
        crate::menus::invoke(&mut app, &ctx, "window.coskitStudio", json!({})).unwrap();
        assert!(!active(&app));
        crate::menus::invoke(&mut app, &ctx, "window.coskitStudio", json!({})).unwrap();
        assert!(active(&app));
    }

    #[test]
    fn inspector_and_layout_never_modify_document_or_versions() {
        let mut app = app();
        let ctx = egui::Context::default();
        app.run("file.new", json!({"width":32,"height":24})).unwrap();
        app.run("coskit.project.checkpoint", json!({"label":"Original"})).unwrap();
        let before = app.session.active().unwrap().doc.clone();
        let project = app.session.active().unwrap().pipeline.clone();
        for id in ["window.toggle.color", "window.toggle.properties", "window.toggle.layers", "window.toggle.history", "window.toggle.navigator"] {
            crate::menus::invoke(&mut app, &ctx, id, json!({})).unwrap();
            assert!(app.ui.panels.dock);
        }
        assert_eq!(before, app.session.active().unwrap().doc);
        assert_eq!(project, app.session.active().unwrap().pipeline);
    }

    #[test]
    fn workspace_renders_all_inspectors_at_common_window_sizes() {
        let mut app = app();
        let ctx = egui::Context::default();
        PhotocraftApp::setup_context(&ctx, crate::theme::ThemeKind::Studio);
        app.run("file.new", json!({"width":80,"height":60})).unwrap();
        for size in [vec2(1024.0, 720.0), vec2(1440.0, 900.0), vec2(1920.0, 1080.0)] {
            for tab in [
                Inspector::Ai,
                Inspector::Adjust,
                Inspector::Layers,
                Inspector::Color,
                Inspector::History,
                Inspector::Navigator,
                Inspector::Character,
                Inspector::Parameters,
            ] {
                app.ui.studio.inspector = tab;
                app.ui.studio.pinned = Some(Inspector::Layers);
                let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), ..Default::default() };
                let mut output = ctx.run_ui(input, |ui| {
                    header(&mut app, ui);
                    sidebar(&mut app, ui);
                    crate::pipeline_ui::timeline(&mut app, ui);
                    assert!(ui.available_width() >= 240.0);
                    assert!(ui.available_height() >= 240.0);
                });
                output.textures_delta.clear();
            }
        }
    }
}
