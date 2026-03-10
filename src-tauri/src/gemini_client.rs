use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use rand::Rng;
use serde_json::{json, Value};

use crate::image_utils;
use crate::settings;

pub const DEFAULT_TEXT_MODEL: &str = "gemini-3.1-pro-preview";
pub const DEFAULT_IMAGE_MODEL: &str = "gemini-3.1-pro-image-preview";

const PERMANENT_ERROR_KEYWORDS: &[&str] =
    &["PROHIBITED_CONTENT", "SAFETY", "RECITATION", "BLOCKED"];

// ---------------------------------------------------------------------------
// Gemini client singleton
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct GeminiClients {
    text_client: reqwest::Client,
    image_client: reqwest::Client,
    text_url: String,
    image_url: String,
    text_api_key: String,
    image_api_key: String,
    pub text_model: String,
    pub image_model: String,
    pub prompts: HashMap<String, String>,
}

static CLIENTS: std::sync::OnceLock<RwLock<Option<GeminiClients>>> = std::sync::OnceLock::new();

fn clients_lock() -> &'static RwLock<Option<GeminiClients>> {
    CLIENTS.get_or_init(|| RwLock::new(None))
}

/// Get a clone of the current clients (drops the lock immediately).
fn get_clients() -> Result<GeminiClients, String> {
    let lock = clients_lock().read().map_err(|e| e.to_string())?;
    lock.as_ref()
        .cloned()
        .ok_or_else(|| "Gemini clients not initialized".to_string())
}

impl GeminiClients {
    pub fn init() -> Result<(), String> {
        crate::dotenv::load_dotenv_files();
        let settings = settings::load_settings();

        // Text model config: settings → .env fallback
        let text_url_raw = if !settings.text_base_url.trim().is_empty() {
            settings.text_base_url.trim().to_string()
        } else {
            crate::dotenv::get_env_var("GEMINI_BASE_URL")
                .trim()
                .to_string()
        };
        let text_api_key = if !settings.text_api_key.trim().is_empty() {
            settings.text_api_key.trim().to_string()
        } else {
            crate::dotenv::get_env_var("GEMINI_API_KEY")
                .trim()
                .to_string()
        };
        let text_timeout = settings.text_timeout_ms;

        // Image model config: settings → .env fallback
        let image_url_raw = if !settings.image_base_url.trim().is_empty() {
            settings.image_base_url.trim().to_string()
        } else {
            crate::dotenv::get_env_var("GEMINI_IMAGE_BASE_URL")
                .trim()
                .to_string()
        };
        let image_api_key = if !settings.image_api_key.trim().is_empty() {
            settings.image_api_key.trim().to_string()
        } else if !text_api_key.is_empty() {
            text_api_key.clone()
        } else {
            String::new()
        };
        let image_timeout = settings.image_timeout_ms;

        if text_api_key.is_empty() {
            return Err("missing API key".to_string());
        }

        // Parse proxy URLs
        let (text_base, url_text_model) = if !text_url_raw.is_empty() {
            parse_proxy_url(&text_url_raw)
        } else {
            (String::new(), String::new())
        };
        let (image_base, url_image_model) = if !image_url_raw.is_empty() {
            parse_proxy_url(&image_url_raw)
        } else {
            (String::new(), String::new())
        };

        // Model priority: settings field > URL-parsed > default
        let text_model = if !settings.text_model.trim().is_empty() {
            settings.text_model.trim().to_string()
        } else if !url_text_model.is_empty() {
            url_text_model
        } else {
            DEFAULT_TEXT_MODEL.to_string()
        };
        let image_model = if !settings.image_model.trim().is_empty() {
            settings.image_model.trim().to_string()
        } else if !url_image_model.is_empty() {
            url_image_model
        } else {
            DEFAULT_IMAGE_MODEL.to_string()
        };

        // Build full API URLs
        let text_url = build_api_url(&text_base, &text_model);
        let image_url = build_api_url(&image_base, &image_model);

        // Build HTTP clients with timeouts
        let text_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(text_timeout))
            .build()
            .map_err(|e| format!("failed to build text client: {e}"))?;

        let image_client = if image_base == text_base
            && image_api_key == text_api_key
            && image_timeout == text_timeout
        {
            text_client.clone()
        } else {
            reqwest::Client::builder()
                .timeout(Duration::from_millis(image_timeout))
                .build()
                .map_err(|e| format!("failed to build image client: {e}"))?
        };

        let prompts = settings.prompts;

        eprintln!("[CosKit] text  → model={text_model}");
        eprintln!("[CosKit] image → model={image_model}");

        let clients = GeminiClients {
            text_client,
            image_client,
            text_url,
            image_url,
            text_api_key,
            image_api_key,
            text_model,
            image_model,
            prompts,
        };

        let mut lock = clients_lock().write().map_err(|e| e.to_string())?;
        *lock = Some(clients);
        Ok(())
    }

    pub fn reset() {
        if let Ok(mut lock) = clients_lock().write() {
            *lock = None;
        }
    }
}

fn build_api_url(base: &str, model: &str) -> String {
    if base.is_empty() {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
        )
    } else {
        format!("{base}/v1beta/models/{model}:generateContent")
    }
}

/// Parse a full proxy URL into (base_url, model_name).
/// Example: "https://yunwu.ai/v1beta/models/gemini-3.1-pro-preview:generateContent"
///       -> ("https://yunwu.ai", "gemini-3.1-pro-preview")
pub fn parse_proxy_url(full_url: &str) -> (String, String) {
    // Extract scheme://host
    let base = if let Some(idx) = full_url.find("://") {
        let after_scheme = &full_url[idx + 3..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        full_url[..idx + 3 + host_end].to_string()
    } else {
        return (full_url.to_string(), String::new());
    };

    // Extract model name from /models/<model_name>:...
    if let Some(idx) = full_url.find("/models/") {
        let after = &full_url[idx + 8..]; // skip "/models/"
        let model = if let Some(colon_idx) = after.find(':') {
            &after[..colon_idx]
        } else if let Some(slash_idx) = after.find('/') {
            &after[..slash_idx]
        } else {
            after
        };
        return (base, model.to_string());
    }

    (full_url.to_string(), String::new())
}

// ---------------------------------------------------------------------------
// Core API call functions
// ---------------------------------------------------------------------------

/// POST JSON to Gemini REST endpoint with exponential backoff retry.
async fn call_with_retry(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    contents: Value,
    config: Value,
    max_tries: u32,
) -> Result<Value, String> {
    let mut tries = 0u32;
    let mut last_error = String::new();

    let body = json!({
        "contents": contents,
        "generationConfig": config,
    });

    let full_url = format!("{url}?key={api_key}");

    while tries < max_tries {
        match client.post(&full_url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                if status.is_success() {
                    return serde_json::from_str(&text)
                        .map_err(|e| format!("JSON parse error: {e}"));
                }

                let err_upper = text.to_uppercase();
                if PERMANENT_ERROR_KEYWORDS
                    .iter()
                    .any(|kw| err_upper.contains(kw))
                {
                    eprintln!("  permanent error (not retrying): {text}");
                    return Err(format!("permanent API error: {text}"));
                }

                last_error = format!("HTTP {status}: {text}");
            }
            Err(e) => {
                last_error = e.to_string();
            }
        }

        tries += 1;
        let wait =
            (2.0f64.powi(tries as i32)).min(10.0) + rand::thread_rng().gen_range(0.0..1.0);
        eprintln!("  retry {tries}/{max_tries} after {wait:.1}s: {last_error}");
        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
    }

    Err(format!(
        "model call failed after {max_tries} tries: {last_error}"
    ))
}

/// Extract text from Gemini API response.
fn extract_text(response: &Value) -> String {
    if let Some(candidates) = response.get("candidates").and_then(|v| v.as_array()) {
        for c in candidates {
            if let Some(parts) = c
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array())
            {
                for p in parts {
                    if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                        let trimmed = t.trim();
                        if !trimmed.is_empty() {
                            return trimmed.to_string();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// Extract image bytes from Gemini API response (inline_data).
fn extract_image_bytes(response: &Value) -> Option<Vec<u8>> {
    if let Some(candidates) = response.get("candidates").and_then(|v| v.as_array()) {
        for c in candidates {
            if let Some(parts) = c
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array())
            {
                for p in parts {
                    if let Some(data_str) = p
                        .get("inlineData")
                        .or_else(|| p.get("inline_data"))
                        .and_then(|d| d.get("data"))
                        .and_then(|d| d.as_str())
                    {
                        if let Ok(bytes) = image_utils::base64_to_bytes(data_str) {
                            return Some(bytes);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse JSON from text, stripping markdown code blocks if present.
fn parse_json(text: &str) -> Result<Value, String> {
    let mut text = text.trim();
    if text.starts_with("```") {
        text = text
            .strip_prefix("```json")
            .or_else(|| text.strip_prefix("```"))
            .unwrap_or(text)
            .trim();
        if text.ends_with("```") {
            text = &text[..text.len() - 3];
            text = text.trim();
        }
    }
    serde_json::from_str(text).map_err(|e| format!("JSON parse error: {e}"))
}

// ---------------------------------------------------------------------------
// Build contents array for Gemini API
// ---------------------------------------------------------------------------

fn build_text_and_image_contents(text: &str, image_b64: &str) -> Value {
    json!([{
        "parts": [
            {"text": text},
            {"inline_data": {"mime_type": "image/jpeg", "data": image_b64}}
        ]
    }])
}

fn text_config(temperature: f64) -> Value {
    json!({
        "temperature": temperature,
        "responseModalities": ["TEXT"]
    })
}

fn image_config(temperature: f64) -> Value {
    json!({
        "temperature": temperature,
        "responseModalities": ["TEXT", "IMAGE"]
    })
}

// ---------------------------------------------------------------------------
// High-level model functions
// Each function clones what it needs from the singleton, then drops the lock
// before any `.await`, making all futures `Send`.
// ---------------------------------------------------------------------------

/// Detect if image is cosplay photography.
pub async fn detect_scene_type(
    image_b64: &str,
    user_prompt: &str,
) -> Result<Value, String> {
    // Clone clients (drops lock immediately)
    let clients = get_clients()?;

    let cosplay_keywords = [
        "cosplay", "cos", "coser", "角色", "二次元", "动漫", "游戏", "原神", "崩坏",
        "星穹铁道", "明日方舟", "fate", "lol", "英雄联盟", "花火", "三月七", "符玄",
        "银狼", "刻晴", "甘雨", "雷电将军",
    ];
    let prompt_lower = user_prompt.to_lowercase();
    let matched: Vec<&str> = if !user_prompt.is_empty() {
        cosplay_keywords
            .iter()
            .filter(|kw| prompt_lower.contains(&kw.to_lowercase()))
            .copied()
            .collect()
    } else {
        Vec::new()
    };
    let keyword_hint = if matched.is_empty() {
        String::new()
    } else {
        format!("用户提及关键词：{}", matched.join("、"))
    };

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("detect_scene_type")
        .unwrap_or_else(|| default_prompts.get("detect_scene_type").unwrap());
    let prompt = tmpl.replace("{{KEYWORD_HINT}}", &keyword_hint);
    let prompt = prompt.trim().to_string();

    let contents = build_text_and_image_contents(&prompt, image_b64);
    let config = text_config(0.1);

    let resp = call_with_retry(
        &clients.text_client,
        &clients.text_url,
        &clients.text_api_key,
        contents,
        config,
        5,
    )
    .await?;

    let text = extract_text(&resp);
    match parse_json(&text) {
        Ok(v) => Ok(v),
        Err(_) => Ok(json!({
            "is_portrait": true,
            "is_cosplay": false,
            "reason": "fallback"
        })),
    }
}

/// Analyze background and recommend replacement.
pub async fn analyze_background(
    image_b64: &str,
    scene: &Value,
    user_prompt: &str,
    bg_prompt: &str,
) -> Result<String, String> {
    let clients = get_clients()?;

    let cosplay_hint = if scene
        .get("is_cosplay")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        "这是一张 cosplay 摄影。"
    } else {
        ""
    };
    let user_bg_hint = if !bg_prompt.is_empty() {
        format!("用户背景偏好：{bg_prompt}")
    } else {
        String::new()
    };
    let user_request_hint = if !user_prompt.is_empty() && bg_prompt.is_empty() {
        format!("用户修图需求（供参考）：{user_prompt}")
    } else {
        String::new()
    };

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("analyze_background")
        .unwrap_or_else(|| default_prompts.get("analyze_background").unwrap());
    let prompt = tmpl
        .replace("{{COSPLAY_HINT}}", cosplay_hint)
        .replace("{{USER_BG_HINT}}", &user_bg_hint)
        .replace("{{USER_REQUEST_HINT}}", &user_request_hint);
    let prompt = prompt.trim().to_string();

    let contents = build_text_and_image_contents(&prompt, image_b64);
    let config = text_config(0.3);

    let resp = call_with_retry(
        &clients.text_client,
        &clients.text_url,
        &clients.text_api_key,
        contents,
        config,
        5,
    )
    .await?;

    let text = extract_text(&resp);
    let result = text.trim();
    if result.is_empty() {
        Ok("保持原背景".to_string())
    } else {
        Ok(result.to_string())
    }
}

/// Main image retouching.
pub async fn retouch_image(
    image_b64: &str,
    user_prompt: &str,
    bg_suggestion: &str,
) -> Result<(Vec<u8>, String), String> {
    let clients = get_clients()?;

    let bg_instruction = if !bg_suggestion.is_empty() && bg_suggestion != "保持原背景" {
        format!(
            "- 背景替换：将背景更换为——{bg_suggestion}；\n\
             - 【透视一致性（关键）】新背景的透视灭点、地平线高度、镜头焦距感必须与原图人物一致，\
             避免人物'悬浮'或比例失调。保持原图的拍摄角度（俯/仰/平视）不变；\n\
             - 新背景与人物的光照方向、色温、景深需自然融合，边缘过渡柔和无硬切割；"
        )
    } else {
        String::new()
    };
    let user_section = if !user_prompt.is_empty() {
        format!("【用户核心需求（最高优先级，必须满足）】\n{user_prompt}")
    } else {
        String::new()
    };

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("retouch_image")
        .unwrap_or_else(|| default_prompts.get("retouch_image").unwrap());
    let prompt = tmpl
        .replace("{{USER_SECTION}}", &user_section)
        .replace("{{BG_INSTRUCTION}}", &bg_instruction);
    let prompt = prompt.trim().to_string();

    let contents = build_text_and_image_contents(&prompt, image_b64);
    let config = image_config(0.3);

    let resp = call_with_retry(
        &clients.image_client,
        &clients.image_url,
        &clients.image_api_key,
        contents,
        config,
        5,
    )
    .await?;

    let mut img_bytes = extract_image_bytes(&resp);
    let mut note = extract_text(&resp);

    // Retry if no image returned
    if img_bytes.is_none() {
        let retry_prompt = format!("{prompt}\n\n注意：必须返回图片。");
        let contents = build_text_and_image_contents(&retry_prompt, image_b64);
        let config = image_config(0.2);

        let resp_retry = call_with_retry(
            &clients.image_client,
            &clients.image_url,
            &clients.image_api_key,
            contents,
            config,
            5,
        )
        .await?;

        img_bytes = extract_image_bytes(&resp_retry);
        let retry_note = extract_text(&resp_retry);
        if !retry_note.is_empty() {
            note = retry_note;
        }
    }

    match img_bytes {
        Some(bytes) => {
            if note.is_empty() {
                note = "已完成人像美化、构图与光线优化".to_string();
            }
            Ok((bytes, note))
        }
        None => {
            let original_bytes = image_utils::base64_to_bytes(image_b64)?;
            Ok((original_bytes, "模型未返回图片，已回退原图".to_string()))
        }
    }
}

/// Apply cosplay special effects.
pub async fn apply_cosplay_effect(
    image_b64: &str,
    effect_prompt: &str,
    user_prompt: &str,
) -> Result<(Vec<u8>, String), String> {
    let clients = get_clients()?;

    let tone_constraint = if !user_prompt.is_empty() {
        format!(
            "\n【用户色调偏好（必须遵守，不得添加与之冲突的色调/光效）】\n{user_prompt}\n"
        )
    } else {
        String::new()
    };
    let effect_text = if effect_prompt.is_empty() {
        "根据画面自动判断"
    } else {
        effect_prompt
    };

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("apply_cosplay_effect")
        .unwrap_or_else(|| default_prompts.get("apply_cosplay_effect").unwrap());
    let prompt = tmpl
        .replace("{{TONE_CONSTRAINT}}", &tone_constraint)
        .replace("{{EFFECT_PROMPT}}", effect_text);
    let prompt = prompt.trim().to_string();

    let contents = build_text_and_image_contents(&prompt, image_b64);
    let config = json!({
        "responseModalities": ["TEXT", "IMAGE"]
    });

    let resp = call_with_retry(
        &clients.image_client,
        &clients.image_url,
        &clients.image_api_key,
        contents,
        config,
        5,
    )
    .await?;

    let img_bytes =
        extract_image_bytes(&resp).ok_or("no image returned in cosplay effect step")?;

    Ok((img_bytes, "已添加轻度氛围特效".to_string()))
}
