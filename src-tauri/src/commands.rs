use std::path::Path;

use serde_json::{json, Value};
use tauri::State;

use crate::engine::{self, AppState};
use crate::gemini_client::GeminiClients;
use crate::image_utils;
use crate::models::{EditNode, PipelineModules, ReferenceImage};
use crate::settings;

fn node_to_value(node: &EditNode) -> Value {
    node.to_dict()
}

fn sniff_image_mime(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png".into())
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg".into())
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp".into())
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif".into())
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        // HEIC/HEIF/AVIF — engine downstream will normalize
        let brand = &bytes[8..12];
        if brand == b"heic" || brand == b"heix" || brand == b"mif1" || brand == b"msf1" {
            Some("image/heic".into())
        } else if brand == b"avif" {
            Some("image/avif".into())
        } else {
            None
        }
    } else {
        None
    }
}

#[tauri::command]
pub async fn pick_image(app: tauri::AppHandle) -> Result<Value, String> {
    use tauri_plugin_dialog::DialogExt;
    use tauri_plugin_fs::FsExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Images", &["jpg", "jpeg", "png", "webp", "gif", "bmp", "heic", "heif", "avif"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    let file = rx.await.map_err(|e| e.to_string())?;

    match file {
        Some(path) => {
            // Extract a display filename. For content:// URIs we may not have one;
            // fall back to a timestamp-based name.
            let filename = match &path {
                tauri_plugin_fs::FilePath::Path(p) => p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "image.jpg".to_string()),
                tauri_plugin_fs::FilePath::Url(url) => {
                    // content://media/...; try to derive a name from the path's last segment
                    url.path_segments()
                        .and_then(|s| s.last().map(|s| s.to_string()))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "image.jpg".to_string())
                }
            };

            // FsExt::read handles both path and content:// URIs on Android
            let data = app
                .fs()
                .read(path)
                .map_err(|e| format!("failed to read file: {e}"))?;

            // Sniff mime from magic bytes; content:// URIs often lack extensions
            let mime = sniff_image_mime(&data).unwrap_or_else(|| {
                let lower = filename.to_lowercase();
                if lower.ends_with(".png") {
                    "image/png"
                } else if lower.ends_with(".webp") {
                    "image/webp"
                } else if lower.ends_with(".gif") {
                    "image/gif"
                } else {
                    "image/jpeg"
                }
                .to_string()
            });

            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &data,
            );
            let data_url = format!("data:{};base64,{}", mime, b64);
            Ok(json!({
                "data_url": data_url,
                "filename": filename,
            }))
        }
        None => Ok(json!({"cancelled": true})),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_session(
    state: State<'_, AppState>,
    image_base64: String,
    filename: String,
) -> Result<Value, String> {
    // Strip data URL prefix if present
    let b64 = if let Some((_, data)) = image_base64.split_once(',') {
        data
    } else {
        &image_base64
    };

    let image_data = image_utils::base64_to_bytes(b64)?;
    let session = engine::create_session(&image_data, &filename)?;

    let root_node = session
        .nodes
        .get(&session.root_id)
        .map(node_to_value)
        .unwrap_or_default();
    let session_id = session.id.clone();

    // Store in app state
    state
        .sessions
        .write()
        .map_err(|e| e.to_string())?
        .insert(session.id.clone(), session);

    Ok(json!({
        "session_id": session_id,
        "root_node": root_node,
    }))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_session(state: State<'_, AppState>, session_id: String) -> Result<Value, String> {
    let sessions = state.sessions.read().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id).ok_or("session not found")?;

    let nodes: serde_json::Map<String, Value> = session
        .nodes
        .iter()
        .map(|(id, n)| (id.clone(), node_to_value(n)))
        .collect();

    Ok(json!({
        "session_id": session.id,
        "root_id": session.root_id,
        "nodes": nodes,
        "active_path": session.active_path,
        "original_size": [session.original_size.0, session.original_size.1],
    }))
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Value, String> {
    let sessions = state.sessions.read().map_err(|e| e.to_string())?;
    let mut result: Vec<Value> = sessions
        .values()
        .map(|s| {
            let root_note = s
                .nodes
                .get(&s.root_id)
                .map(|n| n.note.as_str())
                .unwrap_or("");
            json!({
                "session_id": s.id,
                "root_id": s.root_id,
                "created_at": s.created_at,
                "node_count": s.nodes.len(),
                "note": root_note,
            })
        })
        .collect();

    // Sort by created_at descending
    result.sort_by(|a, b| {
        let ta = a.get("created_at").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tb = b.get("created_at").and_then(|v| v.as_f64()).unwrap_or(0.0);
        tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(Value::Array(result))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Value, String> {
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    if !sessions.contains_key(&session_id) {
        return Ok(json!({"ok": false, "error": "session not found"}));
    }
    engine::delete_session_from_disk(&session_id);
    sessions.remove(&session_id);
    Ok(json!({"ok": true}))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn submit_edit(
    state: State<'_, AppState>,
    session_id: String,
    parent_node_id: String,
    prompt: String,
    modules: PipelineModules,
    reference_images: Option<Vec<ReferenceImage>>,
    mask_base64: Option<String>,
) -> Result<Value, String> {
    let reference_images = reference_images.unwrap_or_default();
    let node = engine::submit_edit(
        &state,
        &session_id,
        &parent_node_id,
        &prompt,
        modules,
        reference_images,
        mask_base64,
    )?;

    let active_path = {
        let sessions = state.sessions.read().map_err(|e| e.to_string())?;
        sessions
            .get(&session_id)
            .map(|s| s.active_path.clone())
            .unwrap_or_default()
    };

    Ok(json!({
        "node_id": node.id,
        "status": node.status,
        "active_path": active_path,
    }))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_node_status(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
) -> Result<Value, String> {
    let sessions = state.sessions.read().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id).ok_or("session not found")?;
    let node = session.nodes.get(&node_id).ok_or("node not found")?;

    let mut result = json!({
        "status": node.status,
        "progress_step": node.progress_step,
        "progress_total": node.progress_total,
        "progress_msg": node.progress_msg,
    });

    if node.status == "done" {
        result["note"] = Value::String(node.note.clone());
    } else if node.status == "error" {
        result["error_msg"] = node
            .error_msg
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null);
    }

    // Include workflow data if present
    if let Some(wp) = node.metadata.get("workflow_plan") {
        result["workflow_plan"] = wp.clone();
    }
    if let Some(ws) = node.metadata.get("workflow_status") {
        result["workflow_status"] = ws.clone();
    }
    if let Some(rh) = node.metadata.get("review_history") {
        result["review_history"] = rh.clone();
    }

    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn navigate_branch(
    state: State<'_, AppState>,
    session_id: String,
    parent_node_id: String,
    direction: i32,
) -> Result<Value, String> {
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    let session = sessions.get_mut(&session_id).ok_or("session not found")?;

    let new_path = engine::switch_branch(session, &parent_node_id, direction);
    Ok(json!({"active_path": new_path}))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn goto_node(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
) -> Result<Value, String> {
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    let session = sessions.get_mut(&session_id).ok_or("session not found")?;

    let new_path = engine::goto_node(session, &node_id);
    Ok(json!({"active_path": new_path}))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_image(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
    thumbnail: Option<bool>,
) -> Result<String, String> {
    let sessions = state.sessions.read().map_err(|e| e.to_string())?;
    let session = match sessions.get(&session_id) {
        Some(s) => s,
        None => return Ok(String::new()),
    };
    let node = match session.nodes.get(&node_id) {
        Some(n) => n,
        None => return Ok(String::new()),
    };

    let is_thumb = thumbnail.unwrap_or(true);
    let path = if is_thumb {
        &node.thumbnail_path
    } else {
        &node.image_path
    };

    if path.is_empty() || !Path::new(path).exists() {
        return Ok(String::new());
    }

    image_utils::image_to_base64_url(path)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn export_image(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
) -> Result<Value, String> {
    let sessions = state.sessions.read().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id).ok_or("session not found")?;
    let node = session.nodes.get(&node_id).ok_or("node not found")?;

    let src = &node.image_path;
    if src.is_empty() || !Path::new(src).exists() {
        return Err("image file not found".to_string());
    }

    let src = src.clone();
    // Match the default filename's extension to whatever we have on disk.
    // After the lossless-storage change, edited results are PNG; legacy
    // sessions still have .jpg.
    let src_ext = Path::new(&src)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let default_name = format!("CosKit_{node_id}.{src_ext}");

    // Drop the sessions lock before showing dialog
    drop(sessions);

    use tauri_plugin_dialog::DialogExt;
    let (filter_label, filter_exts): (&str, &[&str]) = if src_ext == "png" {
        ("PNG Image", &["png"])
    } else {
        ("JPEG Image", &["jpg", "jpeg"])
    };
    let file_path = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter(filter_label, filter_exts)
        .blocking_save_file();

    match file_path {
        Some(path) => {
            use std::io::Write;
            use tauri_plugin_fs::{FsExt, OpenOptions};

            // Read source bytes from our private storage
            let data = std::fs::read(&src)
                .map_err(|e| format!("failed to read source image: {e}"))?;

            // Open destination via FsExt — handles both Path (desktop) and
            // content:// URIs (Android Storage Access Framework).
            let mut opts = OpenOptions::new();
            opts.read(false)
                .write(true)
                .create(true)
                .truncate(true);
            let mut file = app
                .fs()
                .open(path.clone(), opts)
                .map_err(|e| format!("failed to open destination: {e}"))?;
            file.write_all(&data)
                .map_err(|e| format!("failed to write image: {e}"))?;
            // Force durable write and release fd. Critical on Android:
            // MediaProvider snapshots the file size at fd close, so we
            // must finish all writes before drop or Files apps will
            // observe a partial size from the index.
            file.sync_all().ok();
            drop(file);

            let display = match &path {
                tauri_plugin_fs::FilePath::Path(p) => p.to_string_lossy().to_string(),
                tauri_plugin_fs::FilePath::Url(u) => u.to_string(),
            };

            Ok(json!({"ok": true, "path": display}))
        }
        None => Ok(json!({"cancelled": true})),
    }
}

#[tauri::command]
pub async fn get_settings() -> Result<Value, String> {
    let s = settings::load_settings();
    serde_json::to_value(s).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_settings(settings_val: Value) -> Result<Value, String> {
    let s: crate::models::Settings =
        serde_json::from_value(settings_val).map_err(|e| format!("invalid settings: {e}"))?;
    settings::save_settings(&s);

    GeminiClients::reset();
    // Re-init in background
    tokio::spawn(async {
        if let Err(e) = GeminiClients::init() {
            eprintln!("[CosKit] Gemini re-init warning: {e}");
        }
    });

    Ok(json!({"ok": true}))
}

#[tauri::command]
pub async fn get_default_settings() -> Result<Value, String> {
    let defaults = settings::default_settings();
    let prompts = settings::default_prompts();
    Ok(json!({
        "settings": defaults,
        "prompts": prompts,
    }))
}

#[tauri::command]
pub async fn get_data_dir() -> Result<Value, String> {
    let current = settings::data_dir();
    let default = settings::default_data_dir();
    let s = settings::load_settings();
    Ok(json!({
        "current_path": current.to_string_lossy(),
        "default_path": default.to_string_lossy(),
        "is_custom": !s.custom_data_dir.is_empty(),
    }))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn change_data_dir(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    new_path: Option<String>,
) -> Result<Value, String> {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = (app, state, new_path);
        return Ok(json!({"ok": false, "error": "mobile platforms use sandboxed storage"}));
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
    let target = match new_path {
        Some(ref p) if !p.is_empty() => p.clone(),
        _ => {
            // Open folder picker
            use tauri_plugin_dialog::DialogExt;
            let folder = app.dialog().file().blocking_pick_folder();
            match folder {
                Some(path) => path
                    .as_path()
                    .ok_or("无效的文件夹路径")?
                    .to_string_lossy()
                    .to_string(),
                None => return Ok(json!({"cancelled": true})),
            }
        }
    };

    let count = settings::migrate_data_dir(&target)?;

    // Save the new custom_data_dir to settings
    let mut s = settings::load_settings();
    s.custom_data_dir = target.clone();
    settings::save_settings(&s);

    // Reload sessions from new location
    let new_sessions = engine::load_all_sessions();
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    *sessions = new_sessions;

    Ok(json!({
        "ok": true,
        "new_path": target,
        "migrated_count": count,
    }))
    }
}

#[tauri::command]
pub async fn reset_data_dir(state: State<'_, AppState>) -> Result<Value, String> {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = state;
        return Ok(json!({"ok": false, "error": "mobile platforms use sandboxed storage"}));
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
    let default_path = settings::default_data_dir();

    // If currently using custom dir, migrate back
    let s = settings::load_settings();
    if !s.custom_data_dir.is_empty() {
        let default_str = default_path.to_string_lossy().to_string();
        settings::migrate_data_dir(&default_str)
            .map_err(|e| format!("迁移回默认目录失败: {e}"))?;
    }

    // Clear custom_data_dir in settings
    settings::set_custom_data_dir("");
    let mut s = settings::load_settings();
    s.custom_data_dir = String::new();
    settings::save_settings(&s);

    // Reload sessions
    let new_sessions = engine::load_all_sessions();
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    *sessions = new_sessions;

    Ok(json!({
        "ok": true,
        "path": default_path.to_string_lossy(),
    }))
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_workflow_status(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
) -> Result<Value, String> {
    let sessions = state.sessions.read().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id).ok_or("session not found")?;
    let node = session.nodes.get(&node_id).ok_or("node not found")?;

    Ok(json!({
        "workflow_plan": node.metadata.get("workflow_plan"),
        "workflow_status": node.metadata.get("workflow_status"),
    }))
}

#[tauri::command]
pub async fn list_skills() -> Result<Value, String> {
    let skills: Vec<Value> = crate::skills::builtin_skills()
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "description": s.description,
                "category": s.category,
            })
        })
        .collect();
    Ok(Value::Array(skills))
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

fn layer_to_value(layer: &crate::models::Layer) -> Value {
    // Small preview thumb (PNG keeps the alpha of partial layers). Generated
    // on demand — stacks are short and the panel fetches lazily.
    let thumb = image_utils::load_image_from_path(&layer.image_path)
        .ok()
        .and_then(|img| {
            let small = image_utils::resize_max_dimension(&img, 96);
            let bytes = image_utils::image_to_png_bytes(&small).ok()?;
            Some(format!(
                "data:image/png;base64,{}",
                image_utils::bytes_to_base64(&bytes)
            ))
        })
        .unwrap_or_default();

    json!({
        "id": layer.id,
        "name": layer.name,
        "kind": layer.kind,
        "opacity": layer.opacity,
        "blend_mode": layer.blend_mode,
        "visible": layer.visible,
        "locked": layer.locked,
        "has_mask": !layer.mask_path.is_empty(),
        "thumb": thumb,
    })
}

/// List a node's layer stack (bottom-to-top). Synthesizes and persists a base
/// layer for legacy nodes so every completed node is layer-capable.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_layers(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
) -> Result<Value, String> {
    let layers = {
        let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
        let session = sessions.get_mut(&session_id).ok_or("session not found")?;
        let node = session.nodes.get_mut(&node_id).ok_or("node not found")?;
        if node.status != "done" {
            return Ok(json!({"layers": [], "reason": "节点尚未完成"}));
        }
        let had_layers = !node.layers.is_empty();
        engine::ensure_layers(node);
        let layers = node.layers.clone();
        if !had_layers && !layers.is_empty() {
            engine::save_session(session);
        }
        layers
    };

    let values: Vec<Value> = layers.iter().map(layer_to_value).collect();
    Ok(json!({"layers": values}))
}

/// Update mutable properties of one layer, then re-flatten the node image.
/// Accepted props: name, opacity (0..1), blend_mode, visible, locked.
#[tauri::command(rename_all = "snake_case")]
pub async fn update_layer(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
    layer_id: String,
    props: Value,
) -> Result<Value, String> {
    engine::modify_layers(&state.sessions, &session_id, &node_id, |layers| {
        let layer = layers
            .iter_mut()
            .find(|l| l.id == layer_id)
            .ok_or("layer not found")?;

        // `locked` itself is always togglable; everything else requires the
        // layer to be unlocked.
        if let Some(locked) = props.get("locked").and_then(|v| v.as_bool()) {
            layer.locked = locked;
        }
        let unlocking_only = props.as_object().map(|o| o.len() == 1 && o.contains_key("locked")).unwrap_or(false);
        if layer.locked && !unlocking_only {
            return Err(format!("图层「{}」已锁定", layer.name));
        }

        if let Some(name) = props.get("name").and_then(|v| v.as_str()) {
            if !name.trim().is_empty() {
                layer.name = name.trim().to_string();
            }
        }
        if let Some(opacity) = props.get("opacity").and_then(|v| v.as_f64()) {
            layer.opacity = (opacity as f32).clamp(0.0, 1.0);
        }
        if let Some(blend) = props.get("blend_mode").and_then(|v| v.as_str()) {
            const MODES: [&str; 4] = ["normal", "multiply", "screen", "overlay"];
            if MODES.contains(&blend) {
                layer.blend_mode = blend.to_string();
            }
        }
        if let Some(visible) = props.get("visible").and_then(|v| v.as_bool()) {
            layer.visible = visible;
        }
        Ok(())
    })?;
    Ok(json!({"ok": true}))
}

/// Move a layer to a new stack index (0 = bottom).
#[tauri::command(rename_all = "snake_case")]
pub async fn reorder_layer(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
    layer_id: String,
    new_index: usize,
) -> Result<Value, String> {
    engine::modify_layers(&state.sessions, &session_id, &node_id, |layers| {
        let from = layers
            .iter()
            .position(|l| l.id == layer_id)
            .ok_or("layer not found")?;
        let to = new_index.min(layers.len() - 1);
        if from != to {
            let layer = layers.remove(from);
            layers.insert(to, layer);
        }
        Ok(())
    })?;
    Ok(json!({"ok": true}))
}

/// Remove a layer from this node's stack. The raster file is kept — it may be
/// shared with other nodes that inherited the same stack.
#[tauri::command(rename_all = "snake_case")]
pub async fn delete_layer(
    state: State<'_, AppState>,
    session_id: String,
    node_id: String,
    layer_id: String,
) -> Result<Value, String> {
    engine::modify_layers(&state.sessions, &session_id, &node_id, |layers| {
        let idx = layers
            .iter()
            .position(|l| l.id == layer_id)
            .ok_or("layer not found")?;
        if layers.len() <= 1 {
            return Err("不能删除最后一个图层".to_string());
        }
        if layers[idx].kind == "base" {
            return Err("基础图层不可删除".to_string());
        }
        if layers[idx].locked {
            return Err(format!("图层「{}」已锁定", layers[idx].name));
        }
        layers.remove(idx);
        Ok(())
    })?;
    Ok(json!({"ok": true}))
}
