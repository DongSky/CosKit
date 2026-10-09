//! Bounded native-resolution observations. Display conversion never changes the source document.
use super::*;
use photocraft_geom::Rect;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl Region {
    pub fn rect(&self, doc: &Document) -> Result<Rect, String> {
        if self.width == 0
            || self.height == 0
            || self.width > 640
            || self.height > 640
            || self.x.checked_add(self.width).is_none_or(|v| v > doc.size.width)
            || self.y.checked_add(self.height).is_none_or(|v| v > doc.size.height)
        {
            return Err("Detail regions must be inside the canvas, 1..640 pixels per side, in original coordinates".into());
        }
        Ok(Rect::from_xywh(self.x as i32, self.y as i32, self.width, self.height))
    }
}
pub(super) fn preview(d: &Document, region: Option<&Region>) -> Result<String, String> {
    let buffer = match region {
        Some(r) => photocraft_compose::render(d, r.rect(d)?),
        None => photocraft_compose::thumbnail_buffer(d, 1600),
    };
    let color = photocraft_engine::color_cmds::ColorState::default();
    let rgba = color.canvas_display(d).map_err(|e| e.to_string())?.to_rgba8(&buffer);
    let image = photocraft_codecs::Image::from_u8(rgba.width, rgba.height, photocraft_codecs::ChannelLayout::Rgba, rgba.pixels).map_err(|e| e.to_string())?;
    let png = photocraft_codecs::encode(&image, photocraft_codecs::Format::Png, &Default::default()).map_err(|e| e.to_string())?;
    Ok(images::bytes_to_base64(&png))
}
pub(super) fn references(regions: &[Region], original: &Document, before: &Document, current: &Document) -> Result<Vec<ReferenceImage>, String> {
    if regions.is_empty() || regions.len() > 3 {
        return Err("Request 1..3 detail regions".into());
    }
    let mut out = Vec::new();
    for region in regions {
        for (label, d) in [("原始", original), ("上一步", before), ("候选", current)] {
            out.push(ReferenceImage {
                description: format!("{label} 1:1 原尺寸局部 x={} y={} width={} height={}；与同坐标裁片对比", region.x, region.y, region.width, region.height),
                data: preview(d, Some(region))?,
            });
        }
    }
    Ok(out)
}
pub(super) fn automatic(d: &Document, metrics: &Value) -> Vec<Region> {
    // Anchor a native-resolution crop on an actually changed pixel, not the centre of a possibly empty bounding box.
    let Some(point) = metrics.get("detail_anchor").and_then(Value::as_array) else { return vec![] };
    let Some(x) = point.first().and_then(Value::as_u64) else { return vec![] };
    let Some(y) = point.get(1).and_then(Value::as_u64) else { return vec![] };
    let width = d.size.width.min(640);
    let height = d.size.height.min(640);
    vec![Region {
        x: (x as u32).saturating_sub(width / 2).min(d.size.width - width),
        y: (y as u32).saturating_sub(height / 2).min(d.size.height - height),
        width,
        height,
    }]
}
