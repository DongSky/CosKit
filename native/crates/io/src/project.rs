//! Version previews share the same colour-managed float composite as the canvas.
use photocraft_doc::Document;
use std::sync::Arc;

pub fn cache_current_preview(doc: &mut Document) -> photocraft_format::Result<()> {
    let Some(p) = doc.pipeline.as_ref() else { return Ok(()) };
    let Some(version) = p.versions.iter().find(|v| Some(&v.id) == p.head.as_ref()) else { return Ok(()) };
    let hash = version.snapshot.clone();
    if p.previews.contains_key(&hash) {
        return Ok(());
    }
    let buf = photocraft_compose::thumbnail_buffer(doc, 384);
    let meta = photocraft_format::preview::PreviewMetadata {
        width: buf.rect.width(),
        height: buf.rect.height(),
        mode: doc.mode,
        depth: doc.depth,
        icc: doc.icc_profile.as_ref().map(|v| (**v).clone()),
    };
    // Previews are optional. A large ICC or non-finite display sample must never
    // prevent saving the authoritative full-precision layers and profile.
    let Ok(bytes) = photocraft_format::preview::encode(&meta, &buf.px) else { return Ok(()) };
    if let Some(p) = &mut doc.pipeline {
        Arc::make_mut(p).previews.insert(hash, Arc::new(bytes));
    }
    Ok(())
}

pub fn prepare_save(doc: &Document, name: &str) -> photocraft_format::Result<Document> {
    let mut prepared = photocraft_format::pipeline::prepare_save(doc, name)?;
    if name.rsplit('.').next().is_some_and(|s| s.eq_ignore_ascii_case("ckpipe")) {
        cache_current_preview(&mut prepared)?;
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_preview_metadata_does_not_block_lossless_project_saving() {
        let mut doc=Document::new("large profile",photocraft_geom::Size::new(8,8),photocraft_doc::ColorMode::Rgb,photocraft_doc::SampleType::F32);
        doc.icc_profile=Some(Arc::new(vec![255;4<<20]));
        let saved=prepare_save(&doc,"work.ckpipe").unwrap();
        assert!(saved.pipeline.as_ref().unwrap().previews.is_empty());
        let bytes=photocraft_format::save_to_bytes(&saved,&Default::default()).unwrap();
        let restored=photocraft_format::load_from_bytes(&bytes).unwrap();
        assert_eq!(restored.icc_profile,doc.icc_profile);
        assert_eq!(restored.depth,doc.depth);
    }
}
