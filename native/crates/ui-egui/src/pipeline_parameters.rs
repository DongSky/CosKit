//! Visual editors for recorded recipes; the recipe is changed only by a user gesture.
use crate::adjust_editors::{self, EditorCx};
use egui::Ui;
use photocraft_doc::ColorMode;
use serde_json::{Value, json};

fn fields(ui: &mut Ui, value: &mut Value, depth: usize) -> bool {
    if depth > 3 {
        ui.weak("复杂参数请使用高级 JSON");
        return false;
    }
    let Some(map) = value.as_object_mut() else { return false };
    let mut changed = false;
    for (key, v) in map {
        ui.push_id(key, |ui| {
            if matches!(key.as_str(), "layer" | "channel" | "seed" | "target" | "document") {
                ui.label(format!("{key}: {v}"));
                return;
            }
            match v {
                Value::Bool(b) => {
                    changed |= ui.checkbox(b, key).changed();
                }
                Value::Number(number) => {
                    let Some(mut n) = number.as_f64() else { return };
                    let range = match key.as_str() {
                        "opacity" | "flow" | "fill" => Some(0.0..=1.0),
                        "density" => Some(0.0..=100.0),
                        "size" => Some(1.0..=5000.0),
                        "radius" | "feather" => Some(0.0..=1000.0),
                        _ => None,
                    };
                    let edited = ui
                        .horizontal(|ui| {
                            ui.label(key);
                            match range {
                                Some(range) => ui.add(egui::Slider::new(&mut n, range).clamping(egui::SliderClamping::Edits)).changed(),
                                None => ui.add(egui::DragValue::new(&mut n).speed(0.1)).changed(),
                            }
                        })
                        .inner;
                    if edited && n.is_finite() {
                        *v = json!(n);
                        changed = true;
                    }
                }
                Value::String(text) if text.starts_with('#') && text.len() == 7 => {
                    if let Ok(rgb) = u32::from_str_radix(&text[1..], 16) {
                        let mut color = [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8];
                        let edit = ui
                            .horizontal(|ui| {
                                ui.label(key);
                                ui.color_edit_button_srgb(&mut color).changed()
                            })
                            .inner;
                        if edit {
                            *text = format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]);
                            changed = true;
                        }
                    }
                }
                Value::Array(array) if key == "color" && (array.len() == 3 || array.len() == 4) => {
                    let rgb: Option<Vec<f32>> =
                        array.iter().map(|v| v.as_f64().filter(|v| v.is_finite() && (0.0..=1.0).contains(v)).map(|v| v as f32)).collect();
                    if let Some(rgb) = rgb {
                        let mut color = [rgb[0], rgb[1], rgb[2]];
                        let edit = ui
                            .horizontal(|ui| {
                                ui.label(key);
                                ui.color_edit_button_rgb(&mut color).changed()
                            })
                            .inner;
                        if edit {
                            for (slot, n) in array.iter_mut().zip(color) {
                                *slot = json!(n);
                            }
                            changed = true;
                        }
                    }
                }
                Value::Object(_) => {
                    ui.collapsing(key, |ui| {
                        changed |= fields(ui, v, depth + 1);
                    });
                }
                _ => {
                    ui.weak(format!("{key}：高级参数"));
                }
            }
        });
    }
    changed
}
pub(super) fn editor(ui: &mut Ui, recipe: &mut String, mode: ColorMode) {
    let Ok(mut steps) = serde_json::from_str::<Vec<Value>>(recipe) else {
        ui.weak("选择一条编辑记录后调整参数。");
        return;
    };
    let mut changed = false;
    for (index, step) in steps.iter_mut().take(128).enumerate() {
        let Some(command) = step.get("command").and_then(Value::as_str).map(str::to_owned) else { continue };
        ui.push_id(("ckpipe-recipe", index, &command), |ui| {
            ui.strong(&command);
            let Some(params) = step.get_mut("params") else { return };
            if let Some(kind) = command.strip_prefix("image.adjustments.").filter(|kind| adjust_editors::has_editor(kind)) {
                let Ok(adjustment) = photocraft_engine::adjust_params::from_params(kind, params, None, mode) else {
                    ui.weak("参数无效，请在高级 JSON 中修正。");
                    return;
                };
                let mut values = photocraft_engine::adjust_params::to_params(&adjustment);
                let cx =
                    EditorCx { mem: ui.id().with("adjust"), hist: None, gray: mode == ColorMode::Grayscale, swatches: [[0.0; 3], [1.0; 3]], dialog: false };
                if adjust_editors::editor(ui, kind, &mut values, &cx).changed
                    && let (Some(target), Some(edited)) = (params.as_object_mut(), values.as_object())
                {
                    target.extend(edited.clone());
                    changed = true;
                }
            } else {
                changed |= fields(ui, params, 0);
            }
            ui.separator();
        });
    }
    if changed && let Ok(text) = serde_json::to_string_pretty(&steps) {
        *recipe = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewing_controls_preserves_recipe_precision_target_and_unknown_parameters() {
        let ctx = egui::Context::default();
        crate::PhotocraftApp::setup_context(&ctx, Default::default());
        for command in ["image.adjustments.curves", "image.adjustments.exposure", "paint.stroke", "layer.setProps"] {
            let mut recipe=serde_json::to_string(&json!([{"command":command,"params":if command=="paint.stroke" {json!({"color":[0.123456789,0.5,0.7,0.25],"flow":0.333333333,"brush":{"size":12.5},"points":[{"x":3,"y":7}]})}else {json!({})},"active_layer":987,"selected_layers":[987]}])).unwrap();
            let original = recipe.clone();
            let mut output = ctx.run_ui(Default::default(), |ui| editor(ui, &mut recipe, ColorMode::Rgb));
            output.textures_delta.clear();
            assert_eq!(recipe, original, "Opening controls must not rewrite data without a gesture");
        }
    }
}
