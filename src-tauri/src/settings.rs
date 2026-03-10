use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::models::Settings;

/// Return the platform-standard data directory for CosKit.
///
/// - macOS:   `~/Library/Application Support/CosKit/`
/// - Windows: `%APPDATA%/CosKit/`
/// - Linux:   `~/.local/share/CosKit/`
///
/// Falls back to `<exe_parent>/data/` if home directory cannot be determined.
pub fn data_dir() -> PathBuf {
    let dir = if cfg!(target_os = "macos") {
        dirs::home_dir().map(|h| h.join("Library/Application Support/CosKit"))
    } else if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .ok()
            .map(|a| PathBuf::from(a).join("CosKit"))
    } else {
        // Linux / other
        dirs::home_dir().map(|h| h.join(".local/share/CosKit"))
    };

    let dir = dir.unwrap_or_else(|| {
        // Fallback: exe_parent/data/
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("data")
    });

    let _ = fs::create_dir_all(&dir);
    dir
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn default_prompts() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(
        "detect_scene_type".to_string(),
        "你是照片风格识别助手。请判断是否为 cosplay 摄影。\n\
         只输出 JSON：\n\
         {\n\
           \"is_portrait\": true/false,\n\
           \"is_cosplay\": true/false,\n\
           \"reason\": \"不超过30字\"\n\
         }\n\
         {{KEYWORD_HINT}}"
            .to_string(),
    );
    m.insert(
        "analyze_background".to_string(),
        "你是专业摄影后期顾问。请仔细观察这张照片，分析：\n\
         1. 人物的服饰、造型、表情和姿态；\n\
         2. 当前背景的优缺点；\n\
         3. 当前照片的拍摄视角（俯拍/仰拍/平视）与大致镜头焦段；\n\
         4. 如果需要更换背景，推荐一个与人物风格最匹配的场景背景，\
         并确保推荐的背景与当前拍摄视角、透视关系兼容。\n\
         \n\
         {{COSPLAY_HINT}}\n\
         {{USER_BG_HINT}}\n\
         {{USER_REQUEST_HINT}}\n\
         \n\
         请用简明中文输出，仅包含推荐的背景描述（一句话，不超过 80 字，需注明适配的视角）。\n\
         如果当前背景已经很合适，输出\"保持原背景\"。"
            .to_string(),
    );
    m.insert(
        "retouch_image".to_string(),
        "使用Nano Banana编辑图像。任务如下：请对输入照片进行专业修图。\n\
         \n\
         {{USER_SECTION}}\n\
         \n\
         【修图安全规范（在满足用户需求的前提下遵守）】\n\
         - 人像美化：肤质自然、保留细节；若用户未指定则保持自然、不过度磨皮；\n\
         - 场景构图优化：微调视觉重心与层次；\n\
         - 光线优化：提升质感与氛围但避免过曝；\n\
         - 整体保持真实，不改变人物身份特征。\n\
         {{BG_INSTRUCTION}}\n\
         \n\
         请直接返回编辑后图片。"
            .to_string(),
    );
    m.insert(
        "apply_cosplay_effect".to_string(),
        "使用Nano Banana编辑图像。任务如下：你是 cosplay 摄影后期师。\
         请在不破坏真实质感前提下，添加轻度特效：\n\
         - 结合场景与角色，加入克制的氛围光、粒子或能量线；\n\
         - 特效强度轻度，优先突出人物主体；\n\
         - 禁止重度滤镜、过曝、夸张变形。\n\
         {{TONE_CONSTRAINT}}\n\
         特效偏好：{{EFFECT_PROMPT}}"
            .to_string(),
    );
    m
}

pub fn default_settings() -> Settings {
    Settings::default()
}

/// Load settings from settings.json, merging with defaults.
pub fn load_settings() -> Settings {
    let defaults = default_settings();
    let path = settings_path();

    if !path.exists() {
        return defaults;
    }

    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(saved) => {
                let mut settings = defaults;
                if let Some(v) = saved.get("text_base_url").and_then(|v| v.as_str()) {
                    settings.text_base_url = v.to_string();
                }
                if let Some(v) = saved.get("text_api_key").and_then(|v| v.as_str()) {
                    settings.text_api_key = v.to_string();
                }
                if let Some(v) = saved.get("text_model").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        settings.text_model = v.to_string();
                    }
                }
                if let Some(v) = saved.get("image_base_url").and_then(|v| v.as_str()) {
                    settings.image_base_url = v.to_string();
                }
                if let Some(v) = saved.get("image_api_key").and_then(|v| v.as_str()) {
                    settings.image_api_key = v.to_string();
                }
                if let Some(v) = saved.get("image_model").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        settings.image_model = v.to_string();
                    }
                }
                if let Some(v) = saved.get("text_timeout_ms").and_then(|v| v.as_u64()) {
                    settings.text_timeout_ms = v;
                }
                if let Some(v) = saved.get("image_timeout_ms").and_then(|v| v.as_u64()) {
                    settings.image_timeout_ms = v;
                }
                if let Some(prompts) = saved.get("prompts").and_then(|v| v.as_object()) {
                    for (k, v) in prompts {
                        if let Some(s) = v.as_str() {
                            settings.prompts.insert(k.clone(), s.to_string());
                        }
                    }
                }
                settings
            }
            Err(e) => {
                eprintln!("[CosKit] warning: failed to parse settings: {e}");
                defaults
            }
        },
        Err(e) => {
            eprintln!("[CosKit] warning: failed to read settings: {e}");
            defaults
        }
    }
}

/// Persist settings to settings.json.
pub fn save_settings(settings: &Settings) {
    let path = settings_path();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}
