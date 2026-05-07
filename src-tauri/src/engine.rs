use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::gemini_client;
use crate::image_utils;
use crate::models::{EditNode, PipelineModules, ReferenceImage, Session};
use crate::planner;
use crate::reviewer;
use crate::settings;
use crate::workflow;

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
    modules: PipelineModules,
    reference_images: Vec<ReferenceImage>,
) -> Result<EditNode, String> {
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    let session = sessions.get_mut(session_id).ok_or("session not found")?;

    let nid = uuid::Uuid::new_v4().to_string()[..12].to_string();
    let mut node = EditNode::new(nid.clone(), Some(parent_node_id.to_string()));
    node.prompt = prompt.to_string();

    // Store reference image thumbnails in metadata for frontend display
    if !reference_images.is_empty() {
        let ref_thumbs: Vec<serde_json::Value> = reference_images
            .iter()
            .filter_map(|r| {
                let bytes = image_utils::base64_to_bytes(&r.data).ok()?;
                let img = image_utils::load_image_from_bytes(&bytes).ok()?;
                let thumb = image_utils::resize_max_dimension(&img, 128);
                let thumb_bytes = image_utils::image_to_jpeg_bytes(&thumb, 75).ok()?;
                let data_url = format!(
                    "data:image/jpeg;base64,{}",
                    image_utils::bytes_to_base64(&thumb_bytes)
                );
                Some(serde_json::json!({
                    "data_url": data_url,
                    "description": r.description,
                }))
            })
            .collect();
        node.metadata.insert(
            "reference_images".to_string(),
            serde_json::Value::Array(ref_thumbs),
        );
    }

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
            modules,
            reference_images,
        )
        .await;
    });

    Ok(node)
}

/// Helper to update a node in the sessions map.
pub fn update_node(
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
pub fn save_session_from_map(sessions: &RwLock<HashMap<String, Session>>, session_id: &str) {
    if let Ok(lock) = sessions.read() {
        if let Some(session) = lock.get(session_id) {
            save_session(session);
        }
    }
}

/// Resize reference images to a reasonable max dimension before sending to API.
fn prepare_reference_images(refs: Vec<ReferenceImage>) -> Vec<ReferenceImage> {
    refs.into_iter()
        .filter_map(|r| {
            let bytes = image_utils::base64_to_bytes(&r.data).ok()?;
            let img = image_utils::load_image_from_bytes(&bytes).ok()?;
            let resized = image_utils::resize_max_dimension(&img, 1024);
            let jpg_bytes = image_utils::image_to_jpeg_bytes(&resized, 85).ok()?;
            Some(ReferenceImage {
                data: image_utils::bytes_to_base64(&jpg_bytes),
                description: r.description,
            })
        })
        .collect()
}

async fn run_edit_pipeline(
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    session_id: String,
    node_id: String,
    parent_image_path: String,
    prompt: String,
    original_size: (u32, u32),
    modules: PipelineModules,
    reference_images: Vec<ReferenceImage>,
) {
    // Set status to processing
    update_node(&sessions, &session_id, &node_id, |node| {
        node.status = "processing".to_string();
    });

    // Prepare reference images (resize to max 1024px)
    let references = prepare_reference_images(reference_images);

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

    // Downscale large images before sending to API to avoid slow uploads
    // and long inference. The output size will still match original_size
    // because openai_client uses original_size to pick output dimensions,
    // and resize_to_original is applied to the result downstream.
    let api_img = image_utils::resize_max_dimension(&parent_img, 2048);
    let img_bytes = match image_utils::image_to_jpeg_bytes(&api_img, 90) {
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
    eprintln!(
        "[CosKit] pipeline: input downscaled {}x{} -> {}x{}, jpeg={} bytes",
        parent_img.width(),
        parent_img.height(),
        api_img.width(),
        api_img.height(),
        img_bytes.len()
    );
    let image_b64 = image_utils::bytes_to_base64(&img_bytes);

    eprintln!("[CosKit] pipeline: prompt={}", prompt);

    let result = if modules.agent_mode {
        eprintln!(
            "[CosKit] pipeline: entering agent mode (combined={}, save_intermediates={})",
            modules.combined_mode, modules.save_intermediates
        );

        update_node(&sessions, &session_id, &node_id, |node| {
            node.progress_msg = "正在规划工作流...".to_string();
        });

        match planner::plan_workflow(&image_b64, &prompt, &references).await {
            Ok(initial_plan) => {
                run_agent_workflow_with_review(
                    &sessions,
                    &session_id,
                    &node_id,
                    &image_b64,
                    &prompt,
                    original_size,
                    &modules,
                    &references,
                    initial_plan,
                )
                .await
            }
            Err(e) => Err(format!("规划失败: {e}")),
        }
    } else {
        eprintln!(
            "[CosKit] pipeline: entering legacy mode (retouch={}, bg={}, fx={})",
            modules.retouch, modules.background, modules.effects
        );
        // Legacy modular pipeline
        run_modular_pipeline(
            &sessions,
            &session_id,
            &node_id,
            &image_b64,
            &prompt,
            original_size,
            &modules,
            &references,
        )
        .await
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
                        node.error_msg = Some(format!("failed to process result image: {e}"));
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

/// Agent workflow with optional review + auto-correction retry loop.
async fn run_agent_workflow_with_review(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    image_b64: &str,
    prompt: &str,
    original_size: (u32, u32),
    modules: &PipelineModules,
    references: &[ReferenceImage],
    initial_plan: planner::WorkflowPlan,
) -> Result<(Vec<u8>, String), String> {
    let review_settings = settings::load_settings();
    let review_enabled = review_settings.review_enabled;
    let auto_correct = review_settings.review_auto_correct;
    let threshold = review_settings.review_threshold;

    let max_attempts = if review_enabled && auto_correct {
        1 + review_settings.review_max_retries
    } else {
        1
    };

    let review_config = reviewer::ReviewConfig {
        provider: review_settings.review_provider.clone(),
        model: review_settings.review_model.clone(),
        base_url: review_settings.review_base_url.clone(),
        api_key: review_settings.review_api_key.clone(),
    };

    let mut current_plan = initial_plan;
    let mut review_history: Vec<serde_json::Value> = Vec::new();

    for attempt in 0..max_attempts {
        if attempt > 0 {
            eprintln!(
                "[CosKit] review: retry attempt {}/{}",
                attempt,
                max_attempts - 1
            );
        }

        // Execute workflow (step-by-step or combined)
        let exec_result = if modules.combined_mode {
            workflow::execute_workflow_combined(
                sessions,
                session_id,
                node_id,
                image_b64,
                original_size,
                &current_plan,
                references,
            )
            .await
        } else {
            workflow::execute_workflow(
                sessions,
                session_id,
                node_id,
                image_b64,
                original_size,
                &current_plan,
                references,
                modules.save_intermediates,
            )
            .await
        };

        let (result_bytes, note) = match exec_result {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        // If review not enabled, accept immediately
        if !review_enabled {
            return Ok((result_bytes, note));
        }

        // Run review
        update_node(sessions, session_id, node_id, |n| {
            n.progress_msg = "正在审核结果...".to_string();
        });

        let result_b64 = image_utils::bytes_to_base64(&result_bytes);

        match reviewer::review_image(
            &review_config,
            image_b64,
            &result_b64,
            prompt,
            &current_plan,
            references,
            threshold,
        )
        .await
        {
            Ok(review) => {
                let review_json = serde_json::to_value(&review).unwrap_or_default();
                review_history.push(serde_json::json!({
                    "attempt": attempt,
                    "review": review_json,
                }));

                update_node(sessions, session_id, node_id, |n| {
                    n.metadata.insert(
                        "review_history".into(),
                        serde_json::Value::Array(review_history.clone()),
                    );
                });

                let is_last = attempt == max_attempts - 1;
                if review.pass || !auto_correct || is_last {
                    let final_note = format!(
                        "{}\n\n审核评分: {:.1}/10{}",
                        note,
                        review.overall_score,
                        if review.pass { "" } else { "（未达标）" }
                    );
                    return Ok((result_bytes, final_note));
                }

                // Re-plan with feedback
                update_node(sessions, session_id, node_id, |n| {
                    n.progress_msg = format!(
                        "审核评分 {:.1}/10，正在优化重试 ({}/{})...",
                        review.overall_score,
                        attempt + 1,
                        max_attempts - 1
                    );
                });

                match planner::plan_workflow_with_feedback(
                    image_b64,
                    prompt,
                    references,
                    &review.feedback,
                    &review.suggestions,
                )
                .await
                {
                    Ok(new_plan) => {
                        current_plan = new_plan;
                    }
                    Err(e) => {
                        let final_note = format!("{}\n重规划失败: {}", note, e);
                        return Ok((result_bytes, final_note));
                    }
                }
            }
            Err(e) => {
                eprintln!("[CosKit] review error (skipping): {e}");
                let final_note = format!("{}\n审核跳过: {}", note, e);
                return Ok((result_bytes, final_note));
            }
        }
    }

    Err("工作流执行超出最大重试次数".to_string())
}

async fn run_modular_pipeline(
    sessions: &RwLock<HashMap<String, Session>>,
    session_id: &str,
    node_id: &str,
    image_b64: &str,
    prompt: &str,
    original_size: (u32, u32),
    modules: &PipelineModules,
    references: &[ReferenceImage],
) -> Result<(Vec<u8>, String), String> {
    let needs_scene = modules.background || modules.effects;
    let needs_bg = modules.background;
    let needs_retouch = modules.retouch || modules.background;
    let needs_effects = modules.effects;

    let total = needs_scene as u32 + needs_bg as u32 + needs_retouch as u32 + needs_effects as u32;
    let mut step = 0u32;

    let mut scene = serde_json::json!({});
    let mut bg_suggestion = String::new();
    let mut result_bytes: Vec<u8> = Vec::new();
    let mut note = String::new();

    // Step: Detect scene type
    if needs_scene {
        step += 1;
        update_node(sessions, session_id, node_id, |node| {
            node.progress_step = step;
            node.progress_total = total;
            node.progress_msg = "正在分析场景...".to_string();
        });
        scene = gemini_client::detect_scene_type(image_b64, prompt, references).await?;

        let scene_clone = scene.clone();
        update_node(sessions, session_id, node_id, |node| {
            node.metadata.insert("scene_info".to_string(), scene_clone);
        });
    }

    // Step: Analyze background
    if needs_bg {
        step += 1;
        update_node(sessions, session_id, node_id, |node| {
            node.progress_step = step;
            node.progress_total = total;
            node.progress_msg = "正在分析背景...".to_string();
        });
        bg_suggestion =
            gemini_client::analyze_background(image_b64, &scene, prompt, "", references).await?;

        let bg_clone = bg_suggestion.clone();
        update_node(sessions, session_id, node_id, |node| {
            node.metadata.insert(
                "bg_suggestion".to_string(),
                serde_json::Value::String(bg_clone),
            );
        });
    }

    // Step: Retouch image
    if needs_retouch {
        step += 1;
        update_node(sessions, session_id, node_id, |node| {
            node.progress_step = step;
            node.progress_total = total;
            node.progress_msg = "正在修图...".to_string();
        });
        // When only background is selected (no retouch), don't pass the user prompt as retouch instruction
        let retouch_prompt = if modules.retouch { prompt } else { "" };
        let (bytes, retouch_note) = gemini_client::retouch_image(
            image_b64,
            retouch_prompt,
            &bg_suggestion,
            references,
            Some(original_size),
        )
        .await?;
        result_bytes = bytes;
        note = retouch_note;
    }

    // Step: Apply effects
    if needs_effects {
        step += 1;
        update_node(sessions, session_id, node_id, |node| {
            node.progress_step = step;
            node.progress_total = total;
            node.progress_msg = "正在添加特效...".to_string();
        });

        // Use retouch result if available, otherwise use original image
        let effect_b64 = if !result_bytes.is_empty() {
            let effect_img = image_utils::load_image_from_bytes(&result_bytes)?;
            let effect_img = image_utils::resize_to_original(&effect_img, original_size);
            let effect_bytes = image_utils::image_to_jpeg_bytes(&effect_img, 90)?;
            image_utils::bytes_to_base64(&effect_bytes)
        } else {
            image_b64.to_string()
        };

        match gemini_client::apply_cosplay_effect(
            &effect_b64,
            "",
            prompt,
            references,
            Some(original_size),
        )
        .await
        {
            Ok((effect_result, effect_note)) => {
                result_bytes = effect_result;
                if note.is_empty() {
                    note = effect_note;
                } else {
                    note = format!("{note}\n{effect_note}");
                }
            }
            Err(e) => {
                if note.is_empty() {
                    note = format!("特效失败: {e}");
                } else {
                    note = format!("{note}\n特效跳过: {e}");
                }
            }
        }
    }

    // If no steps produced an image, return original
    if result_bytes.is_empty() {
        result_bytes = image_utils::base64_to_bytes(image_b64)?;
        if note.is_empty() {
            note = "未执行任何处理步骤".to_string();
        }
    }

    update_node(sessions, session_id, node_id, |node| {
        node.progress_step = total;
        node.progress_total = total;
        node.progress_msg = "保存结果...".to_string();
    });

    Ok((result_bytes, note))
}
