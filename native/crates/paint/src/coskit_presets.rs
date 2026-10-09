//! Native cosplay retouch presets plus CC0 sampled painting tips by David Revoy.
use crate::brush::*;
use crate::tile::GrayTile;

pub fn builtin() -> Vec<BrushPreset> {
    let pressure = Dynamic::controlled(Control::PenPressure);
    let base = BrushSettings {
        size: 24.0,
        pressure_size: false,
        spacing: 0.08,
        shape_dynamics: ShapeDynamics { enabled: true, size: pressure, ..Default::default() },
        ..Default::default()
    };
    let mut presets = Vec::new();
    let mut add =
        |name: &str, brush: BrushSettings| presets.push(BrushPreset { name: name.into(), group: "CosKit · Cosplay Retouch".into(), builtin: true, brush });
    add("CosKit · Hair Single Strand", BrushSettings { size: 3.0, hardness: 0.8, spacing: 0.03, ..base.clone() });
    add("CosKit · Hair Fine Rake", BrushSettings { size: 22.0, tip: TipShape::Sampled(crate::procedural::rake_tip(64, 5, 5)), spacing: 0.03, ..base.clone() });
    add("CosKit · Hair Soft Flyaways", BrushSettings { size: 2.0, hardness: 0.25, opacity: 0.65, ..base.clone() });
    add("CosKit · Soft Dodge Burn", BrushSettings { size: 160.0, hardness: 0.0, flow: 0.05, spacing: 0.08, ..base.clone() });
    add("CosKit · Rim Light", BrushSettings { size: 28.0, hardness: 0.15, flow: 0.15, ..base.clone() });
    add("CosKit · Glow Airbrush", BrushSettings { size: 180.0, hardness: 0.0, flow: 0.08, ..base.clone() });
    add("CosKit · Fabric Detail", BrushSettings { size: 12.0, tip: TipShape::Sampled(crate::procedural::bristle_tip(64, 29)), flow: 0.2, ..base.clone() });
    add(
        "CosKit · Magic Dust",
        BrushSettings {
            size: 9.0,
            hardness: 0.5,
            spacing: 1.4,
            scattering: Scattering { enabled: true, scatter: Dynamic::jitter(2.0), both_axes: true, count: 2, ..Default::default() },
            shape_dynamics: ShapeDynamics { enabled: true, size: Dynamic { jitter: 0.7, minimum: 0.15, ..pressure }, ..Default::default() },
            ..base.clone()
        },
    );
    add("CosKit · Spark Star", BrushSettings { size: 40.0, tip: TipShape::Sampled(crate::procedural::star_tip(64)), spacing: 1.5, ..base.clone() });
    for &(name, width, height, bytes) in crate::coskit_tips::TIPS {
        presets.push(BrushPreset {
            name: name.into(),
            group: "Deevad · Painting Tips (CC0)".into(),
            builtin: true,
            brush: BrushSettings {
                size: 65.0,
                tip: TipShape::Sampled(GrayTile { width, height, data: bytes.iter().map(|&v| u16::from(v) * 257).collect() }),
                spacing: 0.12,
                flow: 0.3,
                shape_dynamics: ShapeDynamics { enabled: true, size: pressure, angle: Dynamic::controlled(Control::Direction), ..Default::default() },
                ..base.clone()
            },
        });
    }
    presets
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_downloaded_tips_are_valid_and_have_paint_coverage() {
        assert_eq!(crate::coskit_tips::TIPS.len(), 19);
        for &(_, w, h, data) in crate::coskit_tips::TIPS {
            assert_eq!(data.len(), (w * h) as usize);
            assert!(data.iter().any(|&v| v > 32));
        }
        let presets = crate::presets::builtin();
        let mut names: Vec<_> = presets.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), presets.len());
        assert_eq!(builtin().len(), 28);
    }
}
