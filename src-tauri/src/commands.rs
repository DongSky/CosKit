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
) -> Result<Value, String> {
    let reference_images = reference_images.unwrap_or_default();
    let node = engine::submit_edit(
        &state,
        &session_id,
        &parent_node_id,
        &prompt,
        modules,
        reference_images,
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
    let default_name = format!("CosKit_{node_id}.jpg");

    // Drop the sessions lock before showing dialog
    drop(sessions);

    use tauri_plugin_dialog::DialogExt;
    let file_path = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter("JPEG Image", &["jpg", "jpeg"])
        .blocking_save_file();

    match file_path {
        Some(path) => {
            let dest = path.as_path().ok_or("invalid save path")?;
            std::fs::copy(&src, dest).map_err(|e| format!("failed to copy image: {e}"))?;
            Ok(json!({"ok": true, "path": dest.to_string_lossy()}))
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
