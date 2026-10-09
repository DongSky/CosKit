//! The AI boundary uses an sRGB proxy; the editable document keeps its layers, profile and depth.
use crate::{CommandSpec, EngineError, Result, Session};
use base64::Engine as _;
use photocraft_cms::{Builtin, Intent};
use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image, SampleType};
use photocraft_color::{ColorMode, PixelFormat};
use photocraft_doc::{Document, Layer, LayerContent};
use serde_json::{Value, json};

fn error(e: impl std::fmt::Display) -> EngineError {
    EngineError::Other(e.to_string())
}

/// Validate the AI boundary without allocating a full-resolution image.
pub fn validate_document(doc: &Document) -> Result<()> {
    if u64::from(doc.size.width) * u64::from(doc.size.height) > 40_000_000 {
        return Err(error("AI requests are limited to 40 megapixels; native editing is unaffected"));
    }
    if !matches!(doc.mode, ColorMode::Rgb | ColorMode::Grayscale | ColorMode::Cmyk | ColorMode::Lab) {
        return Err(error("Convert a copy to RGB, Gray, CMYK or Lab before an AI edit"));
    }
    Ok(())
}

fn png(width: u32, height: u32, pixels: Vec<u8>) -> Result<Vec<u8>> {
    let image = Image::from_raw(width, height, ChannelLayout::Rgba, SampleType::U8, pixels).map_err(error)?;
    photocraft_codecs::encode(&image, Format::Png, &EncodeOptions::default()).map_err(error)
}

/// Runs on the native worker, outside the render loop. No original document is mutated.
pub fn snapshot(doc: &Document) -> Result<(Vec<u8>, Option<Vec<u8>>)> {
    validate_document(doc)?;
    let mut proxy = doc.clone();
    crate::color_cmds::convert_document(&mut proxy, Builtin::Srgb.profile(), Intent::RelativeColorimetric, true)?;
    let rgba = photocraft_compose::flatten(&proxy).to_rgba8();
    let image = png(doc.size.width, doc.size.height, rgba.pixels)?;
    let mask = if let Some(selection) = &doc.selection {
        let mut rgba = Vec::with_capacity(doc.size.width as usize * doc.size.height as usize * 4);
        for y in 0..doc.size.height as i32 {
            for x in 0..doc.size.width as i32 {
                let k = (selection.sample_channel(x, y, 0).clamp(0.0, 1.0) * 255.0).round() as u8;
                // CosKit/OpenAI mask contract: alpha 255 protects, alpha 0 edits.
                // A grayscale coverage image with opaque alpha protects the entire image.
                rgba.extend_from_slice(&[255, 255, 255, 255 - k]);
            }
        }
        Some(png(doc.size.width, doc.size.height, rgba)?)
    } else {
        None
    };
    Ok((image, mask))
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let state = s.active().ok_or(EngineError::NoDocument)?;
    validate_document(&state.doc)?;
    if p.get("document").and_then(Value::as_u64) != Some(state.doc.id.0) || p.get("revision").and_then(Value::as_u64) != Some(state.revision) {
        return Err(error("The document changed while AI was working. Open the result separately or return to the source document."));
    }
    let encoded = p.get("png").and_then(Value::as_str).ok_or_else(|| error("missing PNG result"))?;
    if encoded.len() > 220_000_000 {
        return Err(error("AI result exceeds the size limit"));
    }
    let bytes = base64::prelude::BASE64_STANDARD.decode(encoded).map_err(error)?;
    // Header is checked before decoding/allocating a potentially untrusted model response.
    if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n") {
        return Err(error("AI result is not a PNG"));
    }
    let dimension = |range: std::ops::Range<usize>| -> Option<u32> { Some(u32::from_be_bytes(bytes.get(range)?.try_into().ok()?)) };
    if dimension(16..20) != Some(state.doc.size.width) || dimension(20..24) != Some(state.doc.size.height) {
        return Err(error("AI result dimensions do not match the source document"));
    }
    let mut imported = photocraft_io::import("ai.png", &bytes).map_err(error)?.document;
    crate::color_cmds::convert_document(&mut imported, &crate::color_cmds::document_profile(&state.doc), Intent::RelativeColorimetric, true)?;
    let surface = imported.layers.first().and_then(|l| l.surface()).ok_or_else(|| error("AI image has no pixels"))?;
    let fmt = state.doc.pixel_format();
    let mut surface = surface.convert(PixelFormat::new(fmt.mode, fmt.sample, true));
    if let Some(mask) = &state.doc.selection {
        let rect = state.doc.bounds();
        let channels = surface.format().channels();
        let mut data = surface.read_region(rect);
        let width = rect.width() as usize;
        for (i, pixel) in data.chunks_exact_mut(channels).enumerate() {
            if let Some(alpha) = pixel.last_mut() {
                let coverage = mask.sample_channel((i % width) as i32, (i / width) as i32, 0).clamp(0.0, 1.0);
                *alpha = if p.get("selectionApplied").and_then(Value::as_bool).unwrap_or(false) { alpha.min(coverage) } else { *alpha * coverage };
            }
        }
        surface.write_region(rect, &data);
        surface.prune();
    }
    let name: String = p.get("name").and_then(Value::as_str).unwrap_or("AI edit").chars().take(160).collect();
    let id = s.edit("CosKit AI edit", move |doc, active| {
        let layer = Layer::new(name, LayerContent::Raster(surface));
        let id = layer.id;
        // A generated composite sits above the whole stack, including adjustment layers.
        doc.layers.push(layer);
        *active = Some(id);
        Ok(id)
    })?;
    Ok(json!({"layer":id.0}))
}

/// Commit an isolated, reviewed native document as one undoable operation.
fn apply_harness(s: &mut Session, p: &Value) -> Result<Value> {
    let state = s.active().ok_or(EngineError::NoDocument)?;
    if p.get("document").and_then(Value::as_u64) != Some(state.doc.id.0) || p.get("revision").and_then(Value::as_u64) != Some(state.revision) {
        return Err(error("The source document changed while the workflow was running"));
    }
    let encoded = p.get("pcraft").and_then(Value::as_str).ok_or_else(|| error("missing native workflow result"))?;
    if encoded.len() > 350_000_000 {
        return Err(error("workflow result exceeds 250 MB"));
    }
    let bytes = base64::prelude::BASE64_STANDARD.decode(encoded).map_err(error)?;
    let opts = photocraft_format::LoadOptions { max_total_bytes: 2 << 30, ..Default::default() };
    let mut result = photocraft_format::load_from_bytes_with(&bytes, &opts).map_err(error)?;
    validate_document(&result)?;
    if result.size != state.doc.size || result.mode != state.doc.mode || result.depth != state.doc.depth || result.icc_profile != state.doc.icc_profile {
        return Err(error("workflow must preserve canvas size, mode, depth and profile"));
    }
    result.id = state.doc.id;
    result.name = state.doc.name.clone();
    result.selection = state.doc.selection.clone();
    let count = result.layer_count();
    let mut source = state.project_document();
    photocraft_format::pipeline::checkpoint(
        &mut source,
        "Before AI workflow",
        state.active_layer.map(|v| v.0),
        state.selected_layers().iter().map(|v| v.0).collect(),
    )
    .map_err(error)?;
    photocraft_io::project::cache_current_preview(&mut source).map_err(error)?;
    let mut project = source.pipeline.as_deref().cloned().unwrap_or_default();
    if let Some(workflow) = &result.pipeline {
        let skip = state.pipeline.as_ref().map_or(0, |p| p.operations.len());
        project.operations.extend(workflow.operations.iter().skip(skip).cloned().map(|mut op| {
            op.base = project.head.clone();
            op
        }));
    }
    result.pipeline = Some(std::sync::Arc::new(project));
    let result_active = result.layers.last().map(|l| l.id.0);
    photocraft_format::pipeline::checkpoint(&mut result, "Reviewed AI workflow", result_active, vec![]).map_err(error)?;
    photocraft_io::project::cache_current_preview(&mut result).map_err(error)?;
    s.edit("CosKit reviewed workflow", move |doc, active| {
        *active = result.layers.last().map(|l| l.id);
        *doc = result;
        Ok(())
    })?;
    s.rebalance_memory();
    Ok(json!({"layers":count,"reviewed":true}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "coskit.harness.apply",
            label: "Apply reviewed CosKit workflow",
            menu: &[],
            shortcut: None,
            params: r#"{"document":id,"revision":n,"pcraft":"base64 native document"}"#,
            enabled: |s| s.active().ok_or_else(|| "no document".to_string()).and_then(|d| validate_document(&d.doc).map_err(|e| e.to_string())),
            run: apply_harness,
            journal: false,
        },
        CommandSpec {
            id: "coskit.ai.apply",
            label: "Apply CosKit AI result",
            menu: &[],
            shortcut: None,
            params: r#"{"document":id,"revision":n,"png":"base64 PNG","name":"layer name","selectionApplied":bool=false}"#,
            enabled: |s| s.active().ok_or_else(|| "no document".to_string()).and_then(|d| validate_document(&d.doc).map_err(|e| e.to_string())),
            run: apply,
            journal: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(depth: u32) -> Session {
        let mut s = Session::new();
        s.execute("file.new", json!({"width":8,"height":8,"depth":depth,"background":"white"})).unwrap();
        s
    }
    fn params(s: &Session) -> Value {
        let d = s.active().unwrap();
        json!({"document":d.doc.id.0,"revision":d.revision,"png":base64::prelude::BASE64_STANDARD.encode(png(8,8,[0,0,255,255].repeat(64)).unwrap())})
    }
    #[test]
    fn harness_commit_preserves_embedded_icc_after_profile_cache_is_initialized() {
        let mut s = setup(16);
        std::sync::Arc::make_mut(&mut s.active_mut().unwrap().doc).icc_profile = Some(Builtin::Srgb.profile().to_bytes());
        let state = s.active().unwrap();
        let _ = crate::color_cmds::document_profile(&state.doc).content_hash();
        let bytes = photocraft_format::save_to_bytes(&state.doc, &Default::default()).unwrap();
        let params = json!({"document":state.doc.id.0,"revision":state.revision,"pcraft":base64::prelude::BASE64_STANDARD.encode(bytes)});
        s.execute("coskit.harness.apply", params).unwrap();
        assert_eq!(s.active().unwrap().doc.icc_profile.as_ref().unwrap().as_ref(), Builtin::Srgb.profile().to_bytes().as_ref());
    }
    #[test]
    fn snapshot_exports_selection_as_inverse_protection_alpha() {
        let mut s = setup(8);
        s.execute("select.rect", json!({"x":2,"y":2,"width":3,"height":3})).unwrap();
        let (_, mask) = snapshot(&s.active().unwrap().doc).unwrap();
        let img = photocraft_codecs::decode(&mask.unwrap()).unwrap().to_rgba8();
        assert_eq!(&img[0..4], &[255, 255, 255, 255]);
        assert_eq!(&img[(3 * 8 + 3) * 4..(3 * 8 + 3) * 4 + 4], &[255, 255, 255, 0]);
    }
    #[test]
    fn apply_preserves_document_depth_layers_and_undo() {
        for depth in [8, 16, 32] {
            let mut s = setup(depth);
            let original = s.active().unwrap().doc.clone();
            let p = params(&s);
            s.execute("coskit.ai.apply", p).unwrap();
            let d = &s.active().unwrap().doc;
            assert_eq!(d.depth, original.depth);
            assert_eq!(d.layers.len(), original.layers.len() + 1);
            assert_eq!(d.layers[0], original.layers[0]);
            s.execute("edit.undo", json!({})).unwrap();
            assert_eq!(s.active().unwrap().doc.layers, original.layers);
            s.execute("edit.redo", json!({})).unwrap();
            assert_eq!(s.active().unwrap().doc.layers.len(), original.layers.len() + 1);
        }
    }
    #[test]
    fn selection_is_preserved_and_protected_pixels_remain_identical() {
        let mut s = setup(8);
        s.execute("select.rect", json!({"x":2,"y":2,"width":3,"height":3})).unwrap();
        let original = photocraft_compose::flatten(&s.active().unwrap().doc);
        s.execute("coskit.ai.apply", params(&s)).unwrap();
        let after = photocraft_compose::flatten(&s.active().unwrap().doc);
        assert_eq!(original.get(0, 0), after.get(0, 0));
        assert!(after.get(3, 3)[2] > 0.9 && after.get(3, 3)[0] < 0.1);
        assert!(s.active().unwrap().doc.selection.is_some());
    }
    #[test]
    fn stale_or_malformed_results_do_not_change_history() {
        let mut s = setup(8);
        let mut p = params(&s);
        p["revision"] = json!(999);
        let rev = s.active().unwrap().revision;
        assert!(s.execute("coskit.ai.apply", p).is_err());
        assert!(s.execute("coskit.ai.apply", json!({})).is_err());
        assert_eq!(s.active().unwrap().revision, rev);
    }

    #[test]
    fn preselected_alpha_is_not_feathered_twice() {
        let mut s = setup(16);
        s.edit("selection", |doc, _| {
            let mut mask = photocraft_raster::Surface::new(PixelFormat::GRAY8);
            mask.write_region(doc.bounds(), &[0.5; 64]);
            doc.selection = Some(mask);
            Ok(())
        })
        .unwrap();
        let mut p = params(&s);
        p["selectionApplied"] = json!(true);
        p["png"] = json!(base64::prelude::BASE64_STANDARD.encode(png(8, 8, [0, 0, 255, 128].repeat(64)).unwrap()));
        s.execute("coskit.ai.apply", p).unwrap();
        let layer = s.active().unwrap().doc.layers.last().unwrap();
        let alpha = layer.surface().unwrap().sample_channel(3, 3, 3);
        assert!((alpha - 0.5).abs() < 0.01, "feather applied twice: {alpha}");
    }

    #[test]
    fn cmyk_proxy_and_result_do_not_change_original_mode() {
        let mut s = Session::new();
        s.execute("file.new", json!({"width":8,"height":8,"mode":"cmyk","depth":16,"background":"white"})).unwrap();
        let original = s.active().unwrap().doc.clone();
        let (proxy, _) = snapshot(&original).unwrap();
        assert!(proxy.starts_with(b"\x89PNG"));
        s.execute("coskit.ai.apply", params(&s)).unwrap();
        let d = &s.active().unwrap().doc;
        assert_eq!(d.mode, ColorMode::Cmyk);
        assert_eq!(d.depth, original.depth);
        assert_eq!(d.layers[0], original.layers[0]);
    }
    #[test]
    fn harness_commit_preserves_native_layers_and_supports_undo_redo() {
        for depth in [8, 16, 32] {
            let mut live = setup(depth);
            let original = live.active().unwrap().doc.clone();
            let mut worker = Session::new();
            worker.add_document((*original).clone(), None);
            worker.execute("layer.new.layer", json!({"name":"Harness paint"})).unwrap();
            worker.execute("paint.stroke", json!({"points":[[3,3,1]],"size":3,"color":"#00ffff"})).unwrap();
            let result = worker.active().unwrap().doc.clone();
            let bytes = photocraft_format::save_to_bytes(&result, &Default::default()).unwrap();
            let p = json!({"document":original.id.0,"revision":live.active().unwrap().revision,"pcraft":base64::prelude::BASE64_STANDARD.encode(bytes)});
            live.execute("coskit.harness.apply", p.clone()).unwrap();
            assert_eq!(live.active().unwrap().doc.layers, result.layers);
            assert!(live.execute("coskit.harness.apply", p).is_err());
            live.execute("edit.undo", json!({})).unwrap();
            assert_eq!(live.active().unwrap().doc.layers, original.layers);
            live.execute("edit.redo", json!({})).unwrap();
            assert_eq!(live.active().unwrap().doc.layers, result.layers);
        }
    }
    #[test]
    fn harness_rejects_malformed_and_resized_bundles_without_edits() {
        let mut live = setup(8);
        let original = live.active().unwrap().doc.clone();
        assert!(live.execute("coskit.harness.apply", json!({})).is_err());
        let mut bad = (*original).clone();
        bad.size.width += 1;
        let bytes = photocraft_format::save_to_bytes(&bad, &Default::default()).unwrap();
        let p = json!({"document":original.id.0,"revision":live.active().unwrap().revision,"pcraft":base64::prelude::BASE64_STANDARD.encode(bytes)});
        assert!(live.execute("coskit.harness.apply", p).is_err());
        assert_eq!(live.active().unwrap().doc, original);
    }
}
