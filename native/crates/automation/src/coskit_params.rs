//! Typed CosKit tools. Stroke coordinates are document pixels, independent of window zoom.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    /// Pen pressure from 0 (lifted) to 1 (full).
    pub pressure: f64,
    #[serde(default)]
    pub tilt_x: f64,
    #[serde(default)]
    pub tilt_y: f64,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub time_ms: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrushStroke {
    /// Ordered path including the start and end. One point stamps a dab; maximum 10000 points.
    pub points: Vec<Point>,
    /// Diameter in document pixels, 0.5..5000.
    pub size: f64,
    /// #RRGGBB or #RRGGBBAA.
    pub color: String,
    /// 0..1, independent of per-point pressure.
    pub opacity: f64,
    /// Paint flow per dab, 0..1.
    pub flow: f64,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub hardness: Option<f64>,
    /// Dab spacing as a fraction of diameter, 0.01..10.
    #[serde(default)]
    pub spacing: Option<f64>,
    #[serde(default)]
    pub layer: Option<u64>,
    /// Paint the layer mask instead of its pixels.
    #[serde(default)]
    pub mask: bool,
    #[serde(default)]
    pub erase: bool,
    /// Fix the jitter seed for reproducible strokes.
    #[serde(default)]
    pub seed: Option<u64>,
}
impl BrushStroke {
    pub fn params(&self) -> Result<Value, String> {
        range("size", self.size, 0.5, 5000.0)?;
        range("opacity", self.opacity, 0.0, 1.0)?;
        range("flow", self.flow, 0.0, 1.0)?;
        if let Some(v) = self.hardness {
            range("hardness", v, 0.0, 1.0)?;
        }
        if let Some(v) = self.spacing {
            range("spacing", v, 0.01, 10.0)?;
        }
        let hex = self.color.strip_prefix('#').ok_or("color must be #RRGGBB or #RRGGBBAA")?;
        if ![6, 8].contains(&hex.len()) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("color must be #RRGGBB or #RRGGBBAA".into());
        }
        if self.points.is_empty() || self.points.len() > 10_000 {
            return Err("points must contain 1..10000 samples".into());
        }
        let mut last_time = 0.0;
        let mut distance = 0.0;
        for (i, p) in self.points.iter().enumerate() {
            range("x", p.x, -1_000_000.0, 1_000_000.0)?;
            range("y", p.y, -1_000_000.0, 1_000_000.0)?;
            range("pressure", p.pressure, 0.0, 1.0)?;
            range("tilt_x", p.tilt_x, -90.0, 90.0)?;
            range("tilt_y", p.tilt_y, -90.0, 90.0)?;
            range("rotation", p.rotation, -360.0, 360.0)?;
            range("time_ms", p.time_ms, last_time, 3_600_000.0)?;
            last_time = p.time_ms;
            if let Some(prev) = i.checked_sub(1).and_then(|j| self.points.get(j)) {
                distance += (p.x - prev.x).hypot(p.y - prev.y);
            }
        }
        if distance > 300_000.0 {
            return Err("stroke path exceeds 300000 pixels; split it into shorter strokes".into());
        }
        let points: Vec<_> = self.points.iter().map(|p| json!([p.x, p.y, p.pressure, p.tilt_x, p.tilt_y, p.rotation, p.time_ms])).collect();
        let mut v = json!({"points":points,"size":self.size,"color":self.color,"opacity":self.opacity,"flow":self.flow,"erase":self.erase});
        for (key, val) in [
            ("preset", json!(self.preset)),
            ("hardness", json!(self.hardness)),
            ("spacing", json!(self.spacing)),
            ("layer", json!(self.layer)),
            ("seed", json!(self.seed)),
        ] {
            if !val.is_null() {
                v[key] = val;
            }
        }
        if self.mask {
            v["target"] = json!("mask");
        }
        Ok(v)
    }
}
fn range(name: &str, v: f64, lo: f64, hi: f64) -> Result<(), String> {
    if v.is_finite() && (lo..=hi).contains(&v) { Ok(()) } else { Err(format!("{name} must be in {lo}..={hi}")) }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrushList {
    #[serde(default)]
    pub full: bool,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiEdit {
    /// Edit instruction. Uses the current composite and selection; preserves the source layers.
    pub prompt: String,
}
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiOptions {
    pub harness_enabled: Option<bool>,
    pub harness_max_steps: Option<u32>,
    pub harness_max_images: Option<u32>,
    pub harness_max_rollbacks: Option<u32>,
    pub retouch: Option<bool>,
    pub background: Option<bool>,
    pub effects: Option<bool>,
    pub agent_mode: Option<bool>,
    pub combined_mode: Option<bool>,
    pub review_enabled: Option<bool>,
    pub save_intermediates: Option<bool>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiPending {
    /// apply (only if document/revision still match), open (new document), or discard.
    pub action: PendingAction,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PendingAction {
    Apply,
    Open,
    Discard,
}
