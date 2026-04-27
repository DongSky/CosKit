use std::time::Duration;

use base64::Engine;
use rand::Rng;
use serde_json::{json, Value};

pub const DEFAULT_TEXT_MODEL: &str = "gpt-5.5";
pub const DEFAULT_IMAGE_MODEL: &str = "gpt-image-2";
pub const DEFAULT_BASE_URL: &str = "https://yunwu.ai/v1";

const PERMANENT_ERROR_KEYWORDS: &[&str] = &[
    "PROHIBITED_CONTENT",
    "SAFETY",
    "RECITATION",
    "BLOCKED",
    "CONTENT_POLICY",
];

/// Resolve OpenAI base URL: settings → OPENAI_BASE_URL env → default (yunwu.ai).
pub fn resolve_base_url(settings_url: &str) -> String {
    let s = settings_url.trim();
    if !s.is_empty() {
        return s.trim_end_matches('/').to_string();
    }
    let env_val = crate::dotenv::get_env_var("OPENAI_BASE_URL");
    let env_trim = env_val.trim();
    if !env_trim.is_empty() {
        return env_trim.trim_end_matches('/').to_string();
    }
    DEFAULT_BASE_URL.to_string()
}

pub fn resolve_api_key(settings_key: &str) -> String {
    let s = settings_key.trim();
    if !s.is_empty() {
        return s.to_string();
    }
    crate::dotenv::get_env_var("OPENAI_API_KEY").trim().to_string()
}

pub fn resolve_text_model(settings_model: &str) -> String {
    let s = settings_model.trim();
    if !s.is_empty() {
        return s.to_string();
    }
    let env_val = crate::dotenv::get_env_var("OPENAI_MODEL");
    let env_trim = env_val.trim();
    if !env_trim.is_empty() {
        return env_trim.to_string();
    }
    DEFAULT_TEXT_MODEL.to_string()
}

pub fn resolve_image_model(settings_model: &str) -> String {
    let s = settings_model.trim();
    if !s.is_empty() {
        return s.to_string();
    }
    let env_val = crate::dotenv::get_env_var("OPENAI_IMAGE_MODEL");
    let env_trim = env_val.trim();
    if !env_trim.is_empty() {
        return env_trim.to_string();
    }
    DEFAULT_IMAGE_MODEL.to_string()
}

/// Convert Gemini-style `contents` JSON value into OpenAI chat-completion
/// `messages`. Each text part becomes a text content item; each inline_data
/// part becomes an image_url content item with a base64 data URL.
fn gemini_contents_to_openai_messages(contents: &Value) -> Value {
    let mut user_content: Vec<Value> = Vec::new();
    if let Some(arr) = contents.as_array() {
        for entry in arr {
            if let Some(parts) = entry.get("parts").and_then(|p| p.as_array()) {
                for p in parts {
                    if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                        if !t.is_empty() {
                            user_content.push(json!({"type": "text", "text": t}));
                        }
                    }
                    let inline = p.get("inline_data").or_else(|| p.get("inlineData"));
                    if let Some(inl) = inline {
                        let mime = inl
                            .get("mime_type")
                            .or_else(|| inl.get("mimeType"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("image/jpeg");
                        if let Some(data) = inl.get("data").and_then(|d| d.as_str()) {
                            let url = format!("data:{};base64,{}", mime, data);
                            user_content.push(json!({
                                "type": "image_url",
                                "image_url": {"url": url}
                            }));
                        }
                    }
                }
            }
        }
    }
    json!([{"role": "user", "content": user_content}])
}

fn wrap_text_as_gemini(text: &str) -> Value {
    json!({
        "candidates": [{
            "content": {"parts": [{"text": text}]}
        }]
    })
}

fn wrap_text_and_image_as_gemini(text: &str, image_b64: &str) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    if !text.is_empty() {
        parts.push(json!({"text": text}));
    }
    if !image_b64.is_empty() {
        parts.push(json!({
            "inline_data": {"mime_type": "image/png", "data": image_b64}
        }));
    }
    json!({
        "candidates": [{
            "content": {"parts": parts}
        }]
    })
}

async fn post_json_with_retry(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    max_tries: u32,
) -> Result<Value, String> {
    let mut tries = 0u32;
    let mut last_error = String::new();

    while tries < max_tries {
        match client
            .post(url)
            .bearer_auth(api_key)
            .json(body)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    return serde_json::from_str(&text)
                        .map_err(|e| format!("JSON parse error: {e}"));
                }
                let err_upper = text.to_uppercase();
                if PERMANENT_ERROR_KEYWORDS.iter().any(|kw| err_upper.contains(kw)) {
                    eprintln!("  [openai] permanent error: {text}");
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
        eprintln!("  [openai] retry {tries}/{max_tries} after {wait:.1}s: {last_error}");
        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
    }

    Err(format!("openai call failed after {max_tries} tries: {last_error}"))
}

/// Call OpenAI chat-completions for a vision/text request. Returns a
/// Gemini-shape JSON value so existing extractors keep working.
pub async fn call_text(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    contents: Value,
    temperature: f64,
    max_tries: u32,
) -> Result<Value, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let messages = gemini_contents_to_openai_messages(&contents);
    let body = json!({
        "model": model,
        "messages": messages,
        "temperature": temperature,
    });
    let resp = post_json_with_retry(client, &url, api_key, &body, max_tries).await?;
    let text = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    Ok(wrap_text_as_gemini(&text))
}

async fn send_image_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    image_bytes: Option<&[u8]>,
    size: &str,
) -> Result<Value, String> {
    if let Some(bytes) = image_bytes {
        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name("input.jpg")
            .mime_str("image/jpeg")
            .map_err(|e| e.to_string())?;
        let form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .text("prompt", prompt.to_string())
            .text("size", size.to_string())
            .part("image", part);
        let resp = client
            .post(url)
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {e}"))
    } else {
        let body = json!({
            "model": model,
            "prompt": prompt,
            "size": size,
        });
        let resp = client
            .post(url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {e}"))
    }
}

/// Compute an OpenAI-compatible output size from given width and height.
///
/// gpt-image-2 constraints:
///   - Both edges must be multiples of 16px
///   - Max edge <= 3840px
///   - Long/short ratio <= 3:1
///   - Total pixels in [655_360, 8_294_400]
///
/// Strategy: keep original dimensions, round to multiples of 16, then clamp.
fn compute_output_size(w: u32, h: u32) -> String {
    let mut ow = ((w as f64) / 16.0).round() as u32 * 16;
    let mut oh = ((h as f64) / 16.0).round() as u32 * 16;

    ow = ow.clamp(256, 3840);
    oh = oh.clamp(256, 3840);

    // Enforce ratio <= 3:1
    if ow > oh * 3 {
        ow = oh * 3;
    } else if oh > ow * 3 {
        oh = ow * 3;
    }

    // Enforce total pixel bounds
    let total = ow as u64 * oh as u64;
    if total < 655_360 {
        return "1024x1024".to_string();
    }
    if total > 8_294_400 {
        let shrink = (8_294_400.0 / total as f64).sqrt();
        ow = ((ow as f64 * shrink) / 16.0).floor() as u32 * 16;
        oh = ((oh as f64 * shrink) / 16.0).floor() as u32 * 16;
    }

    format!("{ow}x{oh}")
}

/// Detect output size from encoded image bytes (fallback when original_size is unavailable).
fn detect_output_size(image_bytes: &[u8]) -> String {
    let (w, h) = match image::ImageReader::new(std::io::Cursor::new(image_bytes))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok())
    {
        Some(dims) => dims,
        None => return "1024x1024".to_string(),
    };
    compute_output_size(w, h)
}

/// Call OpenAI image generation/edit endpoint. Uses /images/edits when an
/// input image is present, otherwise /images/generations. Returns a
/// Gemini-shape JSON value containing the generated image.
pub async fn call_image(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    contents: Value,
    max_tries: u32,
    original_size: Option<(u32, u32)>,
) -> Result<Value, String> {
    let mut prompt = String::new();
    let mut image_data_b64: Option<String> = None;
    if let Some(arr) = contents.as_array() {
        for entry in arr {
            if let Some(parts) = entry.get("parts").and_then(|p| p.as_array()) {
                for p in parts {
                    if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                        if !t.is_empty() {
                            if !prompt.is_empty() {
                                prompt.push('\n');
                            }
                            prompt.push_str(t);
                        }
                    }
                    if image_data_b64.is_none() {
                        if let Some(data) = p
                            .get("inline_data")
                            .or_else(|| p.get("inlineData"))
                            .and_then(|d| d.get("data"))
                            .and_then(|d| d.as_str())
                        {
                            image_data_b64 = Some(data.to_string());
                        }
                    }
                }
            }
        }
    }

    let image_bytes = match image_data_b64 {
        Some(b64) => Some(
            base64::engine::general_purpose::STANDARD
                .decode(b64.as_bytes())
                .map_err(|e| format!("base64 decode failed: {e}"))?,
        ),
        None => None,
    };

    // Use original_size if provided; otherwise detect from encoded bytes.
    let size = if let Some((ow, oh)) = original_size {
        compute_output_size(ow, oh)
    } else if let Some(ref bytes) = image_bytes {
        detect_output_size(bytes)
    } else {
        "1024x1024".to_string()
    };

    let endpoint = if image_bytes.is_some() {
        format!("{}/images/edits", base_url.trim_end_matches('/'))
    } else {
        format!("{}/images/generations", base_url.trim_end_matches('/'))
    };

    let mut tries = 0u32;
    let mut last_error = String::new();
    while tries < max_tries {
        match send_image_request(
            client,
            &endpoint,
            api_key,
            model,
            &prompt,
            image_bytes.as_deref(),
            &size,
        )
        .await
        {
            Ok(resp) => {
                let first = resp
                    .get("data")
                    .and_then(|d| d.as_array())
                    .and_then(|a| a.first());
                let b64 = first
                    .and_then(|x| x.get("b64_json"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                if !b64.is_empty() {
                    return Ok(wrap_text_and_image_as_gemini("", &b64));
                }
                // Fallback: some providers return a `url` instead of b64.
                if let Some(url) = first.and_then(|x| x.get("url")).and_then(|s| s.as_str()) {
                    match client.get(url).send().await {
                        Ok(r) => match r.bytes().await {
                            Ok(bytes) => {
                                let encoded = base64::engine::general_purpose::STANDARD
                                    .encode(&bytes);
                                return Ok(wrap_text_and_image_as_gemini("", &encoded));
                            }
                            Err(e) => last_error = format!("download image failed: {e}"),
                        },
                        Err(e) => last_error = format!("download image failed: {e}"),
                    }
                } else {
                    return Err(format!("openai image response missing image: {resp}"));
                }
            }
            Err(e) => {
                let upper = e.to_uppercase();
                if PERMANENT_ERROR_KEYWORDS.iter().any(|kw| upper.contains(kw)) {
                    return Err(format!("permanent API error: {e}"));
                }
                last_error = e;
            }
        }
        tries += 1;
        let wait =
            (2.0f64.powi(tries as i32)).min(10.0) + rand::thread_rng().gen_range(0.0..1.0);
        eprintln!("  [openai/image] retry {tries}/{max_tries} after {wait:.1}s: {last_error}");
        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
    }

    Err(format!(
        "openai image call failed after {max_tries} tries: {last_error}"
    ))
}

#[allow(dead_code)]
fn _silence_unused_engine() {
    let _ = base64::engine::general_purpose::STANDARD.encode([]);
}
