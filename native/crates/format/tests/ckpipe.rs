mod common;
use common::rich_doc;
use photocraft_color::{ColorMode, SampleType};
use photocraft_format::{pipeline, *};
use serde_json::json;
use std::sync::Arc;

#[test]
fn ckpipe_all_depths_preserve_rich_layers_parameters_chat_and_branches() {
    for depth in SampleType::ALL {
        for mode in [ColorMode::Rgb, ColorMode::Grayscale, ColorMode::Cmyk, ColorMode::Lab] {
            let mut doc = rich_doc(mode, depth);
            let original = doc.clone();
            pipeline::checkpoint(&mut doc, "Original", None, vec![]).unwrap();
            doc.layers[0].opacity = 0.37;
            let p = Arc::make_mut(doc.pipeline.as_mut().unwrap());
            p.conversations.push(json!({"role":"你","text":"保留皮肤纹理"}));
            p.operations.push(photocraft_doc::pipeline::Operation {
                command: "layer.setProps".into(),
                params: json!({"opacity":0.37}),
                base: Some("v1".into()),
                active_layer: None,
                selected_layers: vec![],
                replayable: true,
            });
            pipeline::checkpoint(&mut doc, "Edit", None, vec![]).unwrap();
            let mut restored = pipeline::restore(&doc, "v1").unwrap().0;
            assert_eq!(restored.layers, original.layers);
            pipeline::checkpoint(&mut restored, "Branch", None, vec![]).unwrap();
            let bytes = pipeline::save(&restored, &Default::default()).unwrap();
            let back = load_from_bytes(&bytes).unwrap();
            assert_eq!(back, restored);
            let p = back.pipeline.as_ref().unwrap();
            assert_eq!(p.versions.len(), 3);
            assert_eq!(p.versions[2].parent.as_deref(), Some("v1"));
            assert_eq!(p.snapshots.len(), 2);
            assert_eq!(p.conversations[0]["text"], "保留皮肤纹理");
            assert_eq!(p.operations[0].params["opacity"], 0.37);
            assert_eq!(pipeline::restore(&back, "v2").unwrap().0.layers[0].opacity, 0.37);
        }
    }
}
#[test]
fn malformed_and_newer_projects_fail_without_decoding_snapshots_recursively() {
    let mut doc = rich_doc(ColorMode::Rgb, SampleType::U8);
    pipeline::checkpoint(&mut doc, "base", None, vec![]).unwrap();
    let base = doc.pipeline.as_ref().unwrap();
    let mut bad = (**base).clone();
    bad.schema = 999;
    assert!(matches!(pipeline::validate(&bad), Err(FormatError::TooNew { .. })));
    let mut bad = (**base).clone();
    bad.versions[0].parent = Some("v1".into());
    assert!(pipeline::validate(&bad).is_err());
    let mut bad = (**base).clone();
    bad.snapshots.clear();
    assert!(pipeline::validate(&bad).is_err());
    let mut bad = (**base).clone();
    *bad.snapshots.values_mut().next().unwrap() = Arc::new(vec![1, 2, 3]);
    assert!(pipeline::validate(&bad).is_err());
    let nested = save_to_bytes(&doc, &Default::default()).unwrap();
    let hash = blake3::hash(&nested).to_hex().to_string();
    let mut bad = (**base).clone();
    bad.snapshots.clear();
    bad.snapshots.insert(hash.clone(), Arc::new(nested));
    bad.versions[0].snapshot = hash;
    assert!(pipeline::validate(&bad).is_err());
    assert!(pipeline::restore(&doc, "../outside").is_err());
}
#[test]
fn repeated_saves_are_incremental_versions_and_legacy_documents_load() {
    let doc = rich_doc(ColorMode::Rgb, SampleType::U16);
    assert!(load_from_bytes(&save_to_bytes(&doc, &Default::default()).unwrap()).unwrap().pipeline.is_none());
    let first = pipeline::prepare_save(&doc, "work.ckpipe").unwrap();
    let same = pipeline::prepare_save(&first, "work.ckpipe").unwrap();
    assert_eq!(same.pipeline.as_ref().unwrap().versions.len(), 1);
    let mut changed = same;
    changed.layers[0].opacity = 0.11;
    let saved = pipeline::prepare_save(&changed, "work.ckpipe").unwrap();
    assert_eq!(saved.pipeline.as_ref().unwrap().versions.len(), 2);
    assert_eq!(pipeline::restore(&saved, "v1").unwrap().0.layers, doc.layers);
}

#[test]
fn legacy_zip_history_migrates_without_losing_pixels_or_branch_metadata() {
    let original = rich_doc(ColorMode::Lab, SampleType::F32);
    let bytes = save_to_bytes(&original, &Default::default()).unwrap();
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let mut p = photocraft_doc::pipeline::Pipeline { head: Some("v1".into()), ..Default::default() };
    p.snapshots.insert(hash.clone(), Arc::new(bytes));
    p.versions.push(photocraft_doc::pipeline::Version {
        id: "v1".into(),
        parent: None,
        label: "legacy".into(),
        snapshot: hash,
        operation_count: 0,
        active_layer: None,
        selected_layers: vec![],
    });
    let mut legacy = original.clone();
    legacy.pipeline = Some(Arc::new(p));
    let bytes = save_to_bytes(&legacy, &Default::default()).unwrap();
    let mut loaded = load_from_bytes(&bytes).unwrap();
    let migrated_save = pipeline::prepare_save(&loaded, "upgrade.ckpipe").unwrap();
    assert_eq!(migrated_save.pipeline.as_ref().unwrap().schema, 2);
    assert_eq!(pipeline::restore(&loaded, "v1").unwrap().0.layers, original.layers);
    loaded.layers[0].opacity = 0.25;
    pipeline::checkpoint(&mut loaded, "migrated", None, vec![]).unwrap();
    let p = loaded.pipeline.as_ref().unwrap();
    assert_eq!(p.schema, 2);
    assert!(!p.objects.is_empty());
    assert_eq!(p.versions[1].parent.as_deref(), Some("v1"));
    let round = load_from_bytes(&save_to_bytes(&loaded, &Default::default()).unwrap()).unwrap();
    assert_eq!(pipeline::restore(&round, "v1").unwrap().0.layers, original.layers);
    assert_eq!(pipeline::restore(&round, "v2").unwrap().0.layers, loaded.layers);
}

#[test]
fn metadata_versions_reuse_every_object_and_restore_from_directory_and_zip() {
    let mut doc = rich_doc(ColorMode::Rgb, SampleType::U16);
    pipeline::checkpoint(&mut doc, "base", None, vec![]).unwrap();
    let first = doc.pipeline.as_ref().unwrap().objects.clone();
    for i in 1..6 {
        doc.layers[0].opacity = i as f32 / 10.0;
        pipeline::checkpoint(&mut doc, "edit", None, vec![]).unwrap();
    }
    let p = doc.pipeline.as_ref().unwrap();
    assert_eq!(p.objects.len(), first.len());
    for (key, bytes) in &first {
        assert!(Arc::ptr_eq(bytes, p.objects.get(key).unwrap()));
    }
    let dir = common::temp_dir("ckpipe-shared");
    let mut writer = PcraftWriter::default();
    writer.save_dir(&doc, &dir, &Default::default()).unwrap();
    let back = load_path(&dir).unwrap();
    assert_eq!(back, doc);
    // A damaged block is repaired even with the same writer.
    let key = first.keys().next().unwrap();
    std::fs::write(dir.join(key), b"bad").unwrap();
    writer.save_dir(&doc, &dir, &Default::default()).unwrap();
    assert_eq!(load_path(&dir).unwrap(), doc);
    let (bytes, _) = writer.save_zip(&doc, &Default::default()).unwrap();
    let back = load_from_bytes(&bytes).unwrap();
    assert_eq!(back, doc);
    for i in 1..6 {
        assert_eq!(pipeline::restore(&back, &format!("v{}", i + 1)).unwrap().0.layers[0].opacity, i as f32 / 10.0);
    }
}

#[test]
fn shared_pool_rejects_missing_paths_and_corrupted_compressed_content() {
    let mut doc = rich_doc(ColorMode::Rgb, SampleType::U8);
    pipeline::checkpoint(&mut doc, "base", None, vec![]).unwrap();
    let mut missing = doc.clone();
    Arc::make_mut(missing.pipeline.as_mut().unwrap()).objects.pop_first();
    assert!(pipeline::validate(missing.pipeline.as_ref().unwrap()).is_err());
    let mut invalid = doc.clone();
    Arc::make_mut(invalid.pipeline.as_mut().unwrap()).objects.insert("tiles/../bad.zst".into(), Arc::new(vec![]));
    assert!(pipeline::validate(invalid.pipeline.as_ref().unwrap()).is_err());
    let mut corrupt = doc;
    *Arc::make_mut(corrupt.pipeline.as_mut().unwrap()).objects.values_mut().next().unwrap() = Arc::new(vec![1, 2, 3]);
    // Even if both a caller-created manifest and its compressed checksum agree,
    // the loader still checks decompression and the raw content address.
    let bytes = save_to_bytes(&corrupt, &Default::default()).unwrap();
    assert!(load_from_bytes(&bytes).is_err());
}
