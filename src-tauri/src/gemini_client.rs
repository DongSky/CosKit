use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use rand::Rng;
use serde_json::{json, Value};

use crate::image_utils;
use crate::models::ReferenceImage;
use crate::openai_client;
use crate::settings;

pub const DEFAULT_TEXT_MODEL: &str = "gemini-3.1-pro-preview";
pub const DEFAULT_IMAGE_MODEL: &str = "gemini-3.1-pro-image-preview";

pub const PROVIDER_GEMINI: &str = "gemini";
pub const PROVIDER_OPENAI: &str = "openai";

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
    pub text_provider: String,
    pub image_provider: String,
    /// Base URL (without path) used for OpenAI-style endpoints.
    text_openai_base: String,
    image_openai_base: String,
    pub text_model: String,
    pub image_model: String,
    pub prompts: HashMap<String, String>,
}

static CLIENTS: std::sync::OnceLock<RwLock<Option<GeminiClients>>> = std::sync::OnceLock::new();

fn clients_lock() -> &'static RwLock<Option<GeminiClients>> {
    CLIENTS.get_or_init(|| RwLock::new(None))
}

/// Get a clone of the current clients (drops the lock immediately).
pub fn get_clients() -> Result<GeminiClients, String> {
    let lock = clients_lock().read().map_err(|e| e.to_string())?;
    lock.as_ref()
        .cloned()
        .ok_or_else(|| "Gemini clients not initialized".to_string())
}

impl GeminiClients {
    pub fn init() -> Result<(), String> {
        crate::dotenv::load_dotenv_files();
        let settings = settings::load_settings();

        let text_provider = if settings.text_provider.trim().is_empty() {
            PROVIDER_GEMINI.to_string()
        } else {
            settings.text_provider.trim().to_lowercase()
        };
        let image_provider = if settings.image_provider.trim().is_empty() {
            PROVIDER_GEMINI.to_string()
        } else {
            settings.image_provider.trim().to_lowercase()
        };

        // ----- Text model resolution -----
        let (text_url, text_openai_base, text_api_key, text_model) =
            if text_provider == PROVIDER_OPENAI {
                let base = openai_client::resolve_base_url(&settings.text_base_url);
                let key = openai_client::resolve_api_key(&settings.text_api_key);
                let env_model = crate::dotenv::get_env_var("OPENAI_MODEL")
                    .trim()
                    .to_string();
                let model = if !settings.text_model.trim().is_empty() {
                    settings.text_model.trim().to_string()
                } else if !env_model.is_empty() {
                    env_model
                } else {
                    openai_client::DEFAULT_TEXT_MODEL.to_string()
                };
                (String::new(), base, key, model)
            } else {
                // Gemini
                let raw = if !settings.text_base_url.trim().is_empty() {
                    settings.text_base_url.trim().to_string()
                } else {
                    crate::dotenv::get_env_var("GEMINI_BASE_URL")
                        .trim()
                        .to_string()
                };
                let key = if !settings.text_api_key.trim().is_empty() {
                    settings.text_api_key.trim().to_string()
                } else {
                    crate::dotenv::get_env_var("GEMINI_API_KEY")
                        .trim()
                        .to_string()
                };
                let (base, url_model) = if !raw.is_empty() {
                    parse_proxy_url(&raw)
                } else {
                    (String::new(), String::new())
                };
                let model = if !settings.text_model.trim().is_empty() {
                    settings.text_model.trim().to_string()
                } else {
                    let env_val = crate::dotenv::get_env_var("GEMINI_TEXT_MODEL");
                    if !env_val.trim().is_empty() {
                        env_val.trim().to_string()
                    } else if !url_model.is_empty() {
                        url_model
                    } else {
                        DEFAULT_TEXT_MODEL.to_string()
                    }
                };
                let url = build_api_url(&base, &model);
                (url, String::new(), key, model)
            };
        let text_timeout = settings.text_timeout_ms;

        // ----- Image model resolution -----
        let (image_url, image_openai_base, image_api_key, image_model) =
            if image_provider == PROVIDER_OPENAI {
                let base = openai_client::resolve_base_url(&settings.image_base_url);
                let key_settings = settings.image_api_key.trim().to_string();
                let key = if !key_settings.is_empty() {
                    key_settings
                } else {
                    let env_key = crate::dotenv::get_env_var("OPENAI_API_KEY")
                        .trim()
                        .to_string();
                    if !env_key.is_empty() {
                        env_key
                    } else if text_provider == PROVIDER_OPENAI && !text_api_key.is_empty() {
                        text_api_key.clone()
                    } else {
                        String::new()
                    }
                };
                let env_model = crate::dotenv::get_env_var("OPENAI_IMAGE_MODEL")
                    .trim()
                    .to_string();
                let model = if !settings.image_model.trim().is_empty() {
                    settings.image_model.trim().to_string()
                } else if !env_model.is_empty() {
                    env_model
                } else {
                    openai_client::DEFAULT_IMAGE_MODEL.to_string()
                };
                (String::new(), base, key, model)
            } else {
                let raw = if !settings.image_base_url.trim().is_empty() {
                    settings.image_base_url.trim().to_string()
                } else {
                    crate::dotenv::get_env_var("GEMINI_IMAGE_BASE_URL")
                        .trim()
                        .to_string()
                };
                let key = if !settings.image_api_key.trim().is_empty() {
                    settings.image_api_key.trim().to_string()
                } else {
                    let env_val = crate::dotenv::get_env_var("GEMINI_IMAGE_API_KEY");
                    if !env_val.trim().is_empty() {
                        env_val.trim().to_string()
                    } else if text_provider == PROVIDER_GEMINI && !text_api_key.is_empty() {
                        text_api_key.clone()
                    } else {
                        String::new()
                    }
                };
                let (base, url_model) = if !raw.is_empty() {
                    parse_proxy_url(&raw)
                } else {
                    (String::new(), String::new())
                };
                let model = if !settings.image_model.trim().is_empty() {
                    settings.image_model.trim().to_string()
                } else {
                    let env_val = crate::dotenv::get_env_var("GEMINI_IMAGE_MODEL");
                    if !env_val.trim().is_empty() {
                        env_val.trim().to_string()
                    } else if !url_model.is_empty() {
                        url_model
                    } else {
                        DEFAULT_IMAGE_MODEL.to_string()
                    }
                };
                let url = build_api_url(&base, &model);
                (url, String::new(), key, model)
            };
        let image_timeout = settings.image_timeout_ms;

        if text_api_key.is_empty() {
            return Err("missing API key (text)".to_string());
        }
        if image_api_key.is_empty() {
            return Err("missing API key (image)".to_string());
        }

        // Build HTTP clients
        let text_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(text_timeout))
            .build()
            .map_err(|e| format!("failed to build text client: {e}"))?;

        let reuse = text_provider == image_provider
            && text_openai_base == image_openai_base
            && text_url == image_url
            && text_api_key == image_api_key
            && text_timeout == image_timeout;

        let image_client = if reuse {
            text_client.clone()
        } else {
            reqwest::Client::builder()
                .timeout(Duration::from_millis(image_timeout))
                .build()
                .map_err(|e| format!("failed to build image client: {e}"))?
        };

        let prompts = settings.prompts;

        eprintln!("[CosKit] text  → provider={text_provider} model={text_model}");
        eprintln!("[CosKit] image → provider={image_provider} model={image_model}");

        let clients = GeminiClients {
            text_client,
            image_client,
            text_url,
            image_url,
            text_api_key,
            image_api_key,
            text_provider,
            image_provider,
            text_openai_base,
            image_openai_base,
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
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent")
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

// ---------------------------------------------------------------------------
// Provider dispatchers — used by all high-level helpers below.
// ---------------------------------------------------------------------------

async fn dispatch_text(
    clients: &GeminiClients,
    contents: Value,
    temperature: f64,
    max_tries: u32,
) -> Result<Value, String> {
    if clients.text_provider == PROVIDER_OPENAI {
        openai_client::call_text(
            &clients.text_client,
            &clients.text_openai_base,
            &clients.text_api_key,
            &clients.text_model,
            contents,
            temperature,
            max_tries,
        )
        .await
    } else {
        let config = text_config(temperature);
        call_with_retry(
            &clients.text_client,
            &clients.text_url,
            &clients.text_api_key,
            contents,
            config,
            max_tries,
        )
        .await
    }
}

async fn dispatch_image(
    clients: &GeminiClients,
    contents: Value,
    temperature: f64,
    max_tries: u32,
    original_size: Option<(u32, u32)>,
    mask_b64: Option<&str>,
) -> Result<Value, String> {
    if clients.image_provider == PROVIDER_OPENAI {
        openai_client::call_image(
            &clients.image_client,
            &clients.image_openai_base,
            &clients.image_api_key,
            &clients.image_model,
            contents,
            max_tries,
            original_size,
            mask_b64,
        )
        .await
    } else {
        let config = image_config(temperature, original_size);
        call_with_retry(
            &clients.image_client,
            &clients.image_url,
            &clients.image_api_key,
            contents,
            config,
            max_tries,
        )
        .await
    }
}

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
        let wait = (2.0f64.powi(tries as i32)).min(10.0) + rand::thread_rng().gen_range(0.0..1.0);
        eprintln!("  retry {tries}/{max_tries} after {wait:.1}s: {last_error}");
        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
    }

    Err(format!(
        "model call failed after {max_tries} tries: {last_error}"
    ))
}

/// Extract text from Gemini API response.
pub fn extract_text(response: &Value) -> String {
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
pub fn extract_image_bytes(response: &Value) -> Option<Vec<u8>> {
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
pub fn parse_json(text: &str) -> Result<Value, String> {
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
            {"inline_data": {"mime_type": "image/png", "data": image_b64}}
        ]
    }])
}

/// Build contents with the source image and optional reference images interleaved.
pub fn build_contents_with_references(
    text: &str,
    image_b64: &str,
    references: &[ReferenceImage],
) -> Value {
    if references.is_empty() {
        return build_text_and_image_contents(text, image_b64);
    }

    let mut parts = vec![
        json!({"text": text}),
        json!({"inline_data": {"mime_type": "image/png", "data": image_b64}}),
    ];

    for (i, ref_img) in references.iter().enumerate() {
        let desc = if ref_img.description.trim().is_empty() {
            format!("\n参考图 {}：", i + 1)
        } else {
            format!("\n参考图 {}（{}）：", i + 1, ref_img.description.trim())
        };
        parts.push(json!({"text": desc}));
        parts.push(json!({"inline_data": {"mime_type": "image/png", "data": ref_img.data}}));
    }

    json!([{"parts": parts}])
}

/// Generate the reference images hint for prompt templates.
fn reference_images_hint(references: &[ReferenceImage]) -> String {
    if references.is_empty() {
        return String::new();
    }
    let mut hint = String::from(
        "【参考图像】用户附带了参考图像，请根据每张参考图的说明理解其用途，并在处理时参考相关信息。",
    );
    for (i, ref_img) in references.iter().enumerate() {
        if !ref_img.description.trim().is_empty() {
            hint.push_str(&format!(
                "\n  - 参考图 {}：{}",
                i + 1,
                ref_img.description.trim()
            ));
        }
    }
    hint
}

pub fn text_config(temperature: f64) -> Value {
    json!({
        "temperature": temperature,
        "responseModalities": ["TEXT"]
    })
}

pub fn image_config(temperature: f64, original_size: Option<(u32, u32)>) -> Value {
    let mut cfg = json!({
        "temperature": temperature,
        "responseModalities": ["TEXT", "IMAGE"]
    });

    if let Some((w, h)) = original_size {
        if w > 0 && h > 0 {
            let ar = pick_gemini_aspect_ratio(w, h);
            let sz = pick_gemini_image_size(w, h);
            cfg["imageConfig"] = json!({
                "aspectRatio": ar,
                "imageSize": sz,
            });
        }
    }

    cfg
}

/// Gemini 3.1 Flash Image (Nano Banana 2) supported aspect ratios.
/// Values are (label, w/h ratio).
const GEMINI_ASPECT_RATIOS: &[(&str, f64)] = &[
    ("1:1", 1.0),
    ("3:2", 3.0 / 2.0),
    ("2:3", 2.0 / 3.0),
    ("4:3", 4.0 / 3.0),
    ("3:4", 3.0 / 4.0),
    ("5:4", 5.0 / 4.0),
    ("4:5", 4.0 / 5.0),
    ("16:9", 16.0 / 9.0),
    ("9:16", 9.0 / 16.0),
    ("21:9", 21.0 / 9.0),
    ("9:21", 9.0 / 21.0),
    ("4:1", 4.0),
    ("1:4", 0.25),
    ("8:1", 8.0),
    ("1:8", 0.125),
];

/// Pick the closest supported aspect ratio for a given width and height.
/// Comparison is done in log-space so 4:3 vs 3:4 are treated symmetrically.
fn pick_gemini_aspect_ratio(w: u32, h: u32) -> &'static str {
    let target = (w as f64 / h as f64).ln();
    GEMINI_ASPECT_RATIOS
        .iter()
        .min_by(|a, b| {
            let da = (a.1.ln() - target).abs();
            let db = (b.1.ln() - target).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(label, _)| *label)
        .unwrap_or("1:1")
}

/// Pick the smallest imageSize tier that covers the original's longest edge.
/// Tiers (per Nano Banana 2 docs): 1K (~1024px), 2K (~2048px), 4K (~3840px).
fn pick_gemini_image_size(w: u32, h: u32) -> &'static str {
    let max_edge = w.max(h);
    if max_edge > 2048 {
        "4K"
    } else if max_edge > 1024 {
        "2K"
    } else {
        "1K"
    }
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
    references: &[ReferenceImage],
) -> Result<Value, String> {
    // Clone clients (drops lock immediately)
    let clients = get_clients()?;

    let cosplay_keywords = [
        "cosplay",
        "cos",
        "coser",
        "角色",
        "二次元",
        "动漫",
        "游戏",
        "原神",
        "崩坏",
        "星穹铁道",
        "明日方舟",
        "fate",
        "lol",
        "英雄联盟",
        "花火",
        "三月七",
        "符玄",
        "银狼",
        "刻晴",
        "甘雨",
        "雷电将军",
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

    let ref_hint = reference_images_hint(references);

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("detect_scene_type")
        .unwrap_or_else(|| default_prompts.get("detect_scene_type").unwrap());
    let prompt = tmpl
        .replace("{{KEYWORD_HINT}}", &keyword_hint)
        .replace("{{REFERENCE_IMAGES_HINT}}", &ref_hint);
    let prompt = prompt.trim().to_string();

    let contents = build_contents_with_references(&prompt, image_b64, references);

    let resp = dispatch_text(&clients, contents, 0.1, 5).await?;

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
    references: &[ReferenceImage],
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
    let ref_hint = reference_images_hint(references);

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("analyze_background")
        .unwrap_or_else(|| default_prompts.get("analyze_background").unwrap());
    let prompt = tmpl
        .replace("{{COSPLAY_HINT}}", cosplay_hint)
        .replace("{{USER_BG_HINT}}", &user_bg_hint)
        .replace("{{USER_REQUEST_HINT}}", &user_request_hint)
        .replace("{{REFERENCE_IMAGES_HINT}}", &ref_hint);
    let prompt = prompt.trim().to_string();

    let contents = build_contents_with_references(&prompt, image_b64, references);

    let resp = dispatch_text(&clients, contents, 0.3, 5).await?;

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
    references: &[ReferenceImage],
    original_size: Option<(u32, u32)>,
    mask_b64: Option<&str>,
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
    let ref_hint = reference_images_hint(references);

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("retouch_image")
        .unwrap_or_else(|| default_prompts.get("retouch_image").unwrap());
    let prompt = tmpl
        .replace("{{USER_SECTION}}", &user_section)
        .replace("{{BG_INSTRUCTION}}", &bg_instruction)
        .replace("{{REFERENCE_IMAGES_HINT}}", &ref_hint);
    let prompt = prompt.trim().to_string();

    let (prompt, mask_refs) =
        apply_mask_strategy(&clients, &prompt, image_b64, references, mask_b64)?;
    let contents = build_contents_with_references(&prompt, image_b64, &mask_refs);

    let resp = dispatch_image(&clients, contents, 0.3, 5, original_size, mask_b64).await?;

    let mut img_bytes = extract_image_bytes(&resp);
    let mut note = extract_text(&resp);

    // Retry if no image returned (without references to reduce payload)
    if img_bytes.is_none() {
        let retry_prompt = format!("{prompt}\n\n注意：必须返回图片。");
        let contents = build_text_and_image_contents(&retry_prompt, image_b64);

        let resp_retry = dispatch_image(&clients, contents, 0.2, 5, original_size, mask_b64).await?;

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
    references: &[ReferenceImage],
    original_size: Option<(u32, u32)>,
    mask_b64: Option<&str>,
) -> Result<(Vec<u8>, String), String> {
    let clients = get_clients()?;

    let tone_constraint = if !user_prompt.is_empty() {
        format!("\n【用户色调偏好（必须遵守，不得添加与之冲突的色调/光效）】\n{user_prompt}\n")
    } else {
        String::new()
    };
    let effect_text = if effect_prompt.is_empty() {
        "根据画面自动判断"
    } else {
        effect_prompt
    };
    let ref_hint = reference_images_hint(references);

    let default_prompts = settings::default_prompts();
    let tmpl = clients
        .prompts
        .get("apply_cosplay_effect")
        .unwrap_or_else(|| default_prompts.get("apply_cosplay_effect").unwrap());
    let prompt = tmpl
        .replace("{{TONE_CONSTRAINT}}", &tone_constraint)
        .replace("{{EFFECT_PROMPT}}", effect_text)
        .replace("{{REFERENCE_IMAGES_HINT}}", &ref_hint);
    let prompt = prompt.trim().to_string();

    let (prompt, mask_refs) =
        apply_mask_strategy(&clients, &prompt, image_b64, references, mask_b64)?;
    let contents = build_contents_with_references(&prompt, image_b64, &mask_refs);

    let resp = dispatch_image(&clients, contents, 0.3, 5, original_size, mask_b64).await?;
    let img_bytes = extract_image_bytes(&resp).ok_or("no image returned in cosplay effect step")?;

    Ok((img_bytes, "已添加轻度氛围特效".to_string()))
}

// ---------------------------------------------------------------------------
// Generic model call functions (used by planner & workflow)
// ---------------------------------------------------------------------------

/// Apply the provider-specific mask strategy to (prompt, references).
/// - OpenAI: mask is passed natively via multipart in dispatch_image — prompt
///   and references stay unchanged.
/// - Gemini: no native mask support — append a region-constraint instruction
///   to the prompt and attach a red-overlay reference image marking the edit
///   region.
fn apply_mask_strategy(
    clients: &GeminiClients,
    prompt: &str,
    image_b64: &str,
    references: &[ReferenceImage],
    mask_b64: Option<&str>,
) -> Result<(String, Vec<ReferenceImage>), String> {
    let Some(mask) = mask_b64 else {
        return Ok((prompt.to_string(), references.to_vec()));
    };
    if clients.image_provider == PROVIDER_OPENAI {
        return Ok((prompt.to_string(), references.to_vec()));
    }
    let enhanced = format!(
        "{}\n\n【重要】请仅修改图中用红色半透明标记的区域，保持其余部分完全不变。",
        prompt
    );
    let overlay_b64 = image_utils::generate_mask_overlay(image_b64, mask)?;
    let mut refs = references.to_vec();
    refs.push(ReferenceImage {
        data: overlay_b64,
        description: "红色标记区域为需要编辑的部分，请仅修改红色区域".to_string(),
    });
    Ok((enhanced, refs))
}

/// Generic image model call — sends image + prompt, returns result image bytes.
pub async fn call_image_generation(
    image_b64: &str,
    prompt: &str,
    references: &[ReferenceImage],
    temperature: f64,
    original_size: Option<(u32, u32)>,
    mask_b64: Option<&str>,
) -> Result<Vec<u8>, String> {
    let clients = get_clients()?;

    let (final_prompt, final_refs) =
        apply_mask_strategy(&clients, prompt, image_b64, references, mask_b64)?;

    let contents = build_contents_with_references(&final_prompt, image_b64, &final_refs);
    let resp = dispatch_image(&clients, contents, temperature, 5, original_size, mask_b64).await?;

    extract_image_bytes(&resp).ok_or_else(|| "模型未返回图片".to_string())
}

/// Generic text model call — sends image + prompt, returns raw JSON response.
pub async fn call_text_generation(
    image_b64: &str,
    prompt: &str,
    references: &[ReferenceImage],
    temperature: f64,
) -> Result<Value, String> {
    let clients = get_clients()?;
    let contents = build_contents_with_references(prompt, image_b64, references);

    let resp = dispatch_text(&clients, contents, temperature, 5).await?;

    Ok(resp)
}

/// Text model call with explicit provider config, independent of the singleton.
/// Used by the review agent to call a potentially different model/provider.
pub async fn call_text_with_provider(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    contents: Value,
    temperature: f64,
    max_tries: u32,
) -> Result<Value, String> {
    let timeout = {
        let clients = get_clients().ok();
        clients.map(|c| c.text_client.clone()).is_some()
    };
    // Build a one-off client with the text timeout from settings
    let settings = crate::settings::load_settings();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.text_timeout_ms))
        .build()
        .map_err(|e| format!("failed to build review client: {e}"))?;
    let _ = timeout; // suppress unused

    if provider == PROVIDER_OPENAI {
        openai_client::call_text(
            &client,
            base_url,
            api_key,
            model,
            contents,
            temperature,
            max_tries,
        )
        .await
    } else {
        // Gemini provider
        let url = build_api_url(base_url, model);
        let config = text_config(temperature);
        call_with_retry(&client, &url, api_key, contents, config, max_tries).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_picks_closest_supported() {
        // Original failing case: 5776x4336 (3:2.252...) should snap to 4:3 (1.333) — closest.
        assert_eq!(pick_gemini_aspect_ratio(5776, 4336), "4:3");
        assert_eq!(pick_gemini_aspect_ratio(1024, 1024), "1:1");
        assert_eq!(pick_gemini_aspect_ratio(1920, 1080), "16:9");
        assert_eq!(pick_gemini_aspect_ratio(1080, 1920), "9:16");
        assert_eq!(pick_gemini_aspect_ratio(3000, 2000), "3:2");
        assert_eq!(pick_gemini_aspect_ratio(2000, 3000), "2:3");
        assert_eq!(pick_gemini_aspect_ratio(4000, 1000), "4:1");
        assert_eq!(pick_gemini_aspect_ratio(1000, 4000), "1:4");
        assert_eq!(pick_gemini_aspect_ratio(8000, 1000), "8:1");
    }

    #[test]
    fn image_size_picks_smallest_covering_tier() {
        assert_eq!(pick_gemini_image_size(800, 600), "1K");
        assert_eq!(pick_gemini_image_size(1024, 768), "1K");
        assert_eq!(pick_gemini_image_size(1500, 1000), "2K");
        assert_eq!(pick_gemini_image_size(2048, 1536), "2K");
        assert_eq!(pick_gemini_image_size(3000, 2000), "4K");
        assert_eq!(pick_gemini_image_size(5776, 4336), "4K");
    }

    #[test]
    fn image_config_includes_imageconfig_when_size_known() {
        let cfg = image_config(0.3, Some((5776, 4336)));
        let ic = cfg.get("imageConfig").expect("imageConfig must be present");
        assert_eq!(ic.get("aspectRatio").and_then(|v| v.as_str()), Some("4:3"));
        assert_eq!(ic.get("imageSize").and_then(|v| v.as_str()), Some("4K"));
        assert_eq!(
            cfg.get("responseModalities")
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(2)
        );
    }

    #[test]
    fn image_config_omits_imageconfig_when_size_unknown() {
        let cfg = image_config(0.3, None);
        assert!(cfg.get("imageConfig").is_none());
        let cfg_zero = image_config(0.3, Some((0, 100)));
        assert!(cfg_zero.get("imageConfig").is_none());
    }
}
