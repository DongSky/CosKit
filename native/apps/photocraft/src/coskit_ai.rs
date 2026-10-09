//! Native adapter for CosKit's original AI planner, provider clients and review pipeline.
use photocraft_ui_egui::coskit_ai::{Event, Request, Service};
use std::sync::{atomic::Ordering, mpsc};

pub fn service() -> Service {
    if let Some(path) = crate::app_dirs::config_dir() {
        coskit_ai::settings::set_app_data_dir(path);
    }
    coskit_ai::settings::init_custom_data_dir();
    // Environment is loaded before worker threads start (the shared core is Rust 2021).
    coskit_ai::dotenv::load_dotenv_files();
    Service {
        start: Box::new(|request| {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(request, &tx)));
                let error = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(e)) => Some(e),
                    Err(_) => Some("AI worker stopped unexpectedly; your document was not changed".into()),
                };
                if let Some(e) = error {
                    let _ = tx.send(Event::Failed(e));
                }
            });
            rx
        }),
        load_settings: Box::new(|| serde_json::to_value(coskit_ai::settings::load_settings()).unwrap_or_default()),
        save_settings: Box::new(|value| {
            let settings: coskit_ai::models::Settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
            for provider in [&settings.text_provider, &settings.image_provider] {
                if !["openai", "gemini", "qwen"].contains(&provider.as_str()) {
                    return Err("供应商必须是 openai、gemini 或 qwen".into());
                }
            }
            let path = coskit_ai::settings::default_data_dir().join("settings.json");
            let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
            crate::services::write_atomic(&path, &bytes)?;
            coskit_ai::gemini_client::GeminiClients::reset();
            Ok(())
        }),
        pick_references: Box::new(|| {
            let paths =
                rfd::FileDialog::new().set_title("CosKit · 选择参考图").add_filter("Images", &["png", "jpg", "jpeg", "webp"]).pick_files().unwrap_or_default();
            if paths.len() > 8 {
                return Err("一次最多选择 8 张参考图".into());
            }
            paths
                .into_iter()
                .map(|p| {
                    if std::fs::metadata(&p).map_err(|e| e.to_string())?.len() > 20_000_000 {
                        return Err("参考图文件不能超过 20 MB".into());
                    }
                    Ok((p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), std::fs::read(p).map_err(|e| e.to_string())?))
                })
                .collect()
        }),
        load_history: Box::new(|| {
            std::fs::read(coskit_ai::settings::data_dir().join("native-chat.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
        }),
        save_history: Box::new(|messages| {
            let path = coskit_ai::settings::data_dir().join("native-chat.json");
            let bytes = serde_json::to_vec_pretty(messages).map_err(|e| e.to_string())?;
            crate::services::write_atomic(&path, &bytes)
        }),
    }
}

fn run(request: Request, tx: &mpsc::Sender<Event>) -> Result<(), String> {
    if request.references.len() > 8 {
        return Err("最多使用 8 张参考图".into());
    }
    if request.options["harness_enabled"].as_bool().unwrap_or(true) {
        return crate::coskit_harness::run(request, tx);
    }
    let (png, mask) = photocraft_engine::coskit_ai::snapshot(&request.document).map_err(|e| e.to_string())?;
    if request.cancel.load(Ordering::Relaxed) {
        return Err("编辑已取消".into());
    }
    coskit_ai::gemini_client::GeminiClients::init()?;
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let state = coskit_ai::engine::AppState::new();
    let session = coskit_ai::engine::create_session(&png, &request.document.name)?;
    let session_id = session.id.clone();
    let root = session.root_id.clone();
    state.sessions.write().map_err(|e| e.to_string())?.insert(session_id.clone(), session);
    let modules = serde_json::from_value(request.options).map_err(|e| e.to_string())?;
    let refs = request
        .references
        .into_iter()
        .map(|(name, bytes)| coskit_ai::models::ReferenceImage { data: coskit_ai::image_utils::bytes_to_base64(&bytes), description: name })
        .collect();
    runtime.block_on(async {
        let node = coskit_ai::engine::submit_edit(
            &state,
            &session_id,
            &root,
            &request.prompt,
            modules,
            refs,
            mask.as_ref().map(|m| coskit_ai::image_utils::bytes_to_base64(m)),
        )?;
        let mut last = String::new();
        loop {
            if request.cancel.load(Ordering::Relaxed) {
                return Err("编辑已取消".into());
            }
            let node =
                state.sessions.read().map_err(|e| e.to_string())?.get(&session_id).and_then(|s| s.nodes.get(&node.id)).cloned().ok_or("编辑会话已丢失")?;
            if node.status == "done" || node.status == "completed" {
                // The core's composite already includes the feather. Import the isolated
                // edit layer to avoid applying feathering twice in the native compositor.
                let path = if mask.is_some() {
                    node.layers.last().map(|l| l.image_path.as_str()).ok_or("AI result is missing its edit layer")?
                } else {
                    &node.image_path
                };
                let image = coskit_ai::image_utils::load_image_from_path(path)?;
                if image.as_rgba8().is_some_and(|rgba| rgba.pixels().all(|px| px[3] == 0)) {
                    return Err("AI 返回了完全透明的编辑图层，未应用结果；请检查选区或重试。".into());
                }
                let image = coskit_ai::image_utils::resize_to_original(&image, (request.document.size.width, request.document.size.height));
                let png = coskit_ai::image_utils::image_to_png_bytes(&image)?;
                let note = if node.note.trim().is_empty() { "编辑完成，已保留原始文档图层。".into() } else { node.note };
                tx.send(Event::Done { png, note }).map_err(|e| e.to_string())?;
                return Ok(());
            }
            if node.status == "error" || node.status == "failed" {
                return Err(node.error_msg.unwrap_or_else(|| "AI 编辑失败".into()));
            }
            if node.progress_msg != last {
                last = node.progress_msg;
                tx.send(Event::Progress(last.clone())).map_err(|e| e.to_string())?;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    })
}

#[cfg(test)]
mod selection_contract_tests {
    use serde_json::json;

    #[test]
    fn native_selection_survives_shared_pipeline_and_layer_import() {
        for depth in [8, 16, 32] {
            let mut session = photocraft_engine::Session::new();
            session.execute("file.new", json!({"width":8,"height":8,"depth":depth,"background":"white"})).unwrap();
            session.execute("select.rect", json!({"x":2,"y":2,"width":3,"height":3})).unwrap();
            let before = session.active().unwrap().doc.clone();
            let (original, mask) = photocraft_engine::coskit_ai::snapshot(&before).unwrap();
            let original = coskit_ai::image_utils::load_image_from_bytes(&original).unwrap();
            let mask = coskit_ai::image_utils::load_image_from_bytes(&mask.unwrap()).unwrap();
            let mut blue = original.to_rgba8();
            for px in blue.pixels_mut() {
                px.0 = [0, 0, 255, 255];
            }
            let blue = blue.into();
            let edit = coskit_ai::image_utils::extract_edit_layer(&blue, &mask);
            assert_eq!(edit.to_rgba8().get_pixel(3, 3)[3], 255);
            assert_eq!(edit.to_rgba8().get_pixel(0, 0)[3], 0);
            let composite = coskit_ai::image_utils::composite_with_mask(&original, &blue, &mask);
            assert_eq!(composite.to_rgba8().get_pixel(3, 3).0, [0, 0, 255, 255]);
            assert_eq!(composite.to_rgba8().get_pixel(0, 0).0, [255, 255, 255, 255]);
            let png = coskit_ai::image_utils::image_to_png_bytes(&edit).unwrap();
            let state = session.active().unwrap();
            let params =
                json!({"document":state.doc.id.0,"revision":state.revision,"png":coskit_ai::image_utils::bytes_to_base64(&png),"selectionApplied":true});
            session.execute("coskit.ai.apply", params).unwrap();
            let doc = &session.active().unwrap().doc;
            let layer = doc.layers.last().unwrap().surface().unwrap();
            assert!(layer.sample_channel(3, 3, 3) > 0.99);
            assert_eq!(layer.sample_channel(0, 0, 3), 0.0);
            assert_eq!(doc.size, before.size);
            assert_eq!(doc.layers[0], before.layers[0]);
        }
    }
}
