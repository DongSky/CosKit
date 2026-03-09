use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::gemini_client;
use crate::image_utils;
use crate::models::{EditNode, Session};
use crate::settings;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------
pub struct AppState {
    pub sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

fn data_dir() -> PathBuf {
    settings::data_dir()
}

// ---------------------------------------------------------------------------
// Session persistence
// ---------------------------------------------------------------------------
pub fn save_session(session: &Session) {
    let sdir = data_dir().join(&session.id);
    let _ = fs::create_dir_all(&sdir);
    let path = sdir.join("session.json");
    if let Ok(json) = serde_json::to_string_pretty(session) {
        let _ = fs::write(path, json);
    }
}

pub fn load_session(session_id: &str) -> Option<Session> {
    let path = data_dir().join(session_id).join("session.json");
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn load_all_sessions() -> HashMap<String, Session> {
    let dir = data_dir();
    let mut sessions = HashMap::new();
    if !dir.exists() {
        return sessions;
    }
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(s) = load_session(name) {
                        sessions.insert(s.id.clone(), s);
                    }
                }
            }
        }
    }
    sessions
}

pub fn delete_session_from_disk(session_id: &str) -> bool {
    let sdir = data_dir().join(session_id);
    if sdir.exists() && sdir.is_dir() {
        fs::remove_dir_all(sdir).is_ok()
    } else {
        false
    }
}

/// Load all sessions into AppState on startup.
pub fn load_all_sessions_into(state: &AppState) {
    let sessions = load_all_sessions();
    eprintln!("[CosKit] loaded {} session(s)", sessions.len());
    if let Ok(mut lock) = state.sessions.write() {
        *lock = sessions;
    }
}

// ---------------------------------------------------------------------------
// Session creation
// ---------------------------------------------------------------------------
pub fn create_session(image_data: &[u8], filename: &str) -> Result<Session, String> {
    let sid = uuid::Uuid::new_v4().to_string()[..12].to_string();
    let sdir = data_dir().join(&sid);
    fs::create_dir_all(&sdir).map_err(|e| format!("failed to create session dir: {e}"))?;

    let img = image_utils::load_image_from_bytes(image_data)?;
    let original_size = (img.width(), img.height());

    // Save original
    let orig_path = sdir.join("original.jpg");
    image_utils::save_jpeg(&img, &orig_path, 95)?;

    // Save thumbnail
    let thumb_path = sdir.join("original_thumb.jpg");
    image_utils::make_thumbnail(&img, &thumb_path)?;

    // Create root node
    let root_id = uuid::Uuid::new_v4().to_string()[..12].to_string();
    let mut root = EditNode::new(root_id.clone(), None);
    root.image_path = orig_path.to_string_lossy().to_string();
    root.thumbnail_path = thumb_path.to_string_lossy().to_string();
    root.note = format!("{filename} · {}×{}", original_size.0, original_size.1);
    root.status = "done".to_string();

    let mut session = Session::new(sid, root_id.clone(), original_size);
    session.nodes.insert(root_id.clone(), root);
    session.active_path = vec![root_id];

    save_session(&session);
    Ok(session)
}

// ---------------------------------------------------------------------------
// Tree operations
// ---------------------------------------------------------------------------
pub fn compute_active_path(session: &Session, leaf_id: &str) -> Vec<String> {
    let mut path = Vec::new();
    let mut nid = Some(leaf_id.to_string());
    while let Some(id) = nid {
        path.push(id.clone());
        nid = session.nodes.get(&id).and_then(|n| n.parent_id.clone());
    }
    path.reverse();
    path
}

pub fn walk_to_leaf(session: &Session, node_id: &str) -> String {
    let mut nid = node_id.to_string();
    while let Some(node) = session.nodes.get(&nid) {
        if node.children.is_empty() {
            break;
        }
        nid = node.children[0].clone();
    }
    nid
}

pub fn switch_branch(session: &mut Session, parent_id: &str, direction: i32) -> Vec<String> {
    let parent = match session.nodes.get(parent_id) {
        Some(p) => p.clone(),
        None => return session.active_path.clone(),
    };

    if parent.children.len() <= 1 {
        return session.active_path.clone();
    }

    // Find current child in active_path
    let mut current_child = None;
    for (i, nid) in session.active_path.iter().enumerate() {
        if nid == parent_id && i + 1 < session.active_path.len() {
            current_child = Some(session.active_path[i + 1].clone());
            break;
        }
    }
    let current_child = current_child.unwrap_or_else(|| parent.children[0].clone());

    let idx = parent
        .children
        .iter()
        .position(|c| c == &current_child)
        .unwrap_or(0);
    let len = parent.children.len() as i32;
    let new_idx = ((idx as i32 + direction) % len + len) % len;
    let new_child = &parent.children[new_idx as usize];

    let leaf = walk_to_leaf(session, new_child);
    session.active_path = compute_active_path(session, &leaf);
    save_session(session);
    session.active_path.clone()
}

pub fn goto_node(session: &mut Session, node_id: &str) -> Vec<String> {
    let leaf = walk_to_leaf(session, node_id);
    session.active_path = compute_active_path(session, &leaf);
    save_session(session);
    session.active_path.clone()
}

// ---------------------------------------------------------------------------
// Edit pipeline (background task)
// ---------------------------------------------------------------------------
pub fn submit_edit(
    state: &AppState,
    session_id: &str,
    parent_node_id: &str,
    prompt: &str,
) -> Result<EditNode, String> {
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    let session = sessions.get_mut(session_id).ok_or("session not found")?;

    let nid = uuid::Uuid::new_v4().to_string()[..12].to_string();
    let mut node = EditNode::new(nid.clone(), Some(parent_node_id.to_string()));
    node.prompt = prompt.to_string();

    session.nodes.insert(nid.clone(), node.clone());

    // Add child to parent
    if let Some(parent) = session.nodes.get_mut(parent_node_id) {
        parent.children.push(nid.clone());
    }

    // Update active path
    let leaf = walk_to_leaf(session, &nid);
    session.active_path = compute_active_path(session, &leaf);
    save_session(session);

    // Gather data for the background task
    let parent_image_path = session
        .nodes
        .get(parent_node_id)
        .map(|n| n.image_path.clone())
        .unwrap_or_default();
    let original_size = session.original_size;
    let is_first_round = parent_node_id == session.root_id;
    let session_id = session_id.to_string();
    let prompt = prompt.to_string();
    let node_id = nid.clone();

    // Clone the Arc for the background task
    let sessions_arc = Arc::clone(&state.sessions);

    tokio::spawn(async move {
        run_edit_pipeline(
            sessions_arc,
            session_id,
            node_id,
            parent_image_path,
            prompt,
            original_size,
            is_first_round,
        )
        .await;
    });

    Ok(node)
}

/// Helper to update a node in the sessions map.
fn update_node(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    f: impl FnOnce(&mut EditNode),
) {
    if let Ok(mut lock) = sessions.write() {
        if let Some(session) = lock.get_mut(session_id) {
            if let Some(node) = session.nodes.get_mut(node_id) {
                f(node);
            }
        }
    }
}

/// Helper to save session from the sessions map.
fn save_session_from_map(sessions: &RwLock<HashMap<String, Session>>, session_id: &str) {
    if let Ok(lock) = sessions.read() {
        if let Some(session) = lock.get(session_id) {
            save_session(session);
        }
    }
}

async fn run_edit_pipeline(
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    session_id: String,
    node_id: String,
    parent_image_path: String,
    prompt: String,
    original_size: (u32, u32),
    is_first_round: bool,
) {
    // Set status to processing
    update_node(&sessions, &session_id, &node_id, |node| {
        node.status = "processing".to_string();
    });

    // Load parent image and encode to base64
    let parent_img = match image_utils::load_image_from_path(&parent_image_path) {
        Ok(img) => img,
        Err(e) => {
            update_node(&sessions, &session_id, &node_id, |node| {
                node.status = "error".to_string();
                node.error_msg = Some(format!("failed to load parent image: {e}"));
            });
            save_session_from_map(&sessions, &session_id);
            return;
        }
    };

    let img_bytes = match image_utils::image_to_jpeg_bytes(&parent_img, 90) {
        Ok(b) => b,
        Err(e) => {
            update_node(&sessions, &session_id, &node_id, |node| {
                node.status = "error".to_string();
                node.error_msg = Some(format!("failed to encode image: {e}"));
            });
            save_session_from_map(&sessions, &session_id);
            return;
        }
    };
    let image_b64 = image_utils::bytes_to_base64(&img_bytes);

    let result = if is_first_round {
        run_full_pipeline(&sessions, &session_id, &node_id, &image_b64, &prompt, original_size)
            .await
    } else {
        run_light_pipeline(&sessions, &session_id, &node_id, &image_b64, &prompt).await
    };

    match result {
        Ok((result_bytes, note)) => {
            let sdir = data_dir().join(&session_id);
            let img_path = sdir.join(format!("{node_id}.jpg"));
            let thumb_path = sdir.join(format!("{node_id}_thumb.jpg"));

            let result_img = match image_utils::load_image_from_bytes(&result_bytes) {
                Ok(img) => image_utils::resize_to_original(&img, original_size),
                Err(e) => {
                    update_node(&sessions, &session_id, &node_id, |node| {
                        node.status = "error".to_string();
                        node.error_msg =
                            Some(format!("failed to process result image: {e}"));
                    });
                    save_session_from_map(&sessions, &session_id);
                    return;
                }
            };

            if let Err(e) = image_utils::save_jpeg(&result_img, &img_path, 95) {
                update_node(&sessions, &session_id, &node_id, |node| {
                    node.status = "error".to_string();
                    node.error_msg = Some(format!("failed to save image: {e}"));
                });
                save_session_from_map(&sessions, &session_id);
                return;
            }

            let _ = image_utils::make_thumbnail(&result_img, &thumb_path);

            let img_path_str = img_path.to_string_lossy().to_string();
            let thumb_path_str = thumb_path.to_string_lossy().to_string();
            update_node(&sessions, &session_id, &node_id, |node| {
                node.image_path = img_path_str;
                node.thumbnail_path = thumb_path_str;
                node.note = note;
                node.status = "done".to_string();
            });
            save_session_from_map(&sessions, &session_id);
        }
        Err(e) => {
            update_node(&sessions, &session_id, &node_id, |node| {
                node.status = "error".to_string();
                node.error_msg = Some(e.clone());
            });
            save_session_from_map(&sessions, &session_id);
            eprintln!("[CosKit] edit error: {e}");
        }
    }
}

async fn run_full_pipeline(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    image_b64: &str,
    prompt: &str,
    original_size: (u32, u32),
) -> Result<(Vec<u8>, String), String> {
    let total = 4u32;

    // Step 1: Detect scene type
    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = 1;
        node.progress_total = total;
        node.progress_msg = "正在分析场景...".to_string();
    });
    let scene = gemini_client::detect_scene_type(image_b64, prompt).await?;

    let scene_clone = scene.clone();
    update_node(sessions, session_id, node_id, |node| {
        node.metadata
            .insert("scene_info".to_string(), scene_clone);
    });

    // Step 2: Analyze background
    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = 2;
        node.progress_total = total;
        node.progress_msg = "正在分析背景...".to_string();
    });
    let bg_suggestion =
        gemini_client::analyze_background(image_b64, &scene, prompt, "").await?;

    let bg_clone = bg_suggestion.clone();
    update_node(sessions, session_id, node_id, |node| {
        node.metadata.insert(
            "bg_suggestion".to_string(),
            serde_json::Value::String(bg_clone),
        );
    });

    // Step 3: Retouch image
    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = 3;
        node.progress_total = total;
        node.progress_msg = "正在修图...".to_string();
    });
    let (mut result_bytes, mut note) =
        gemini_client::retouch_image(image_b64, prompt, &bg_suggestion).await?;

    // Step 4: Optional cosplay effect
    let is_cosplay = scene
        .get("is_cosplay")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if is_cosplay {
        update_node(sessions, session_id, node_id, |node| {
            node.progress_step = 4;
            node.progress_total = total;
            node.progress_msg = "正在添加特效...".to_string();
        });

        // Re-encode result for effect step
        let effect_img = image_utils::load_image_from_bytes(&result_bytes)?;
        let effect_img = image_utils::resize_to_original(&effect_img, original_size);
        let effect_bytes = image_utils::image_to_jpeg_bytes(&effect_img, 90)?;
        let effect_b64 = image_utils::bytes_to_base64(&effect_bytes);

        match gemini_client::apply_cosplay_effect(&effect_b64, "", prompt).await {
            Ok((effect_result, effect_note)) => {
                result_bytes = effect_result;
                note = format!("{note}\n{effect_note}");
            }
            Err(e) => {
                note = format!("{note}\n特效跳过: {e}");
            }
        }
    }

    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = total;
        node.progress_total = total;
        node.progress_msg = "保存结果...".to_string();
    });

    Ok((result_bytes, note))
}

async fn run_light_pipeline(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    image_b64: &str,
    prompt: &str,
) -> Result<(Vec<u8>, String), String> {
    let total = 2u32;

    // Step 1: Retouch
    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = 1;
        node.progress_total = total;
        node.progress_msg = "正在修图...".to_string();
    });
    let (result_bytes, note) = gemini_client::retouch_image(image_b64, prompt, "").await?;

    // Step 2: Save
    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = 2;
        node.progress_total = total;
        node.progress_msg = "保存结果...".to_string();
    });

    Ok((result_bytes, note))
}
