use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::models::Settings;

static CUSTOM_DATA_DIR: RwLock<Option<String>> = RwLock::new(None);
static APP_DATA_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Set the resolved app data directory (called once at startup from
/// the Tauri builder where an AppHandle is available). On mobile the
/// process HOME points at "/" which is read-only, so we must use the
/// platform-resolved app-private dir instead.
pub fn set_app_data_dir(dir: PathBuf) {
    if let Ok(mut lock) = APP_DATA_DIR.write() {
        *lock = Some(dir);
    }
}

/// Set the custom data dir override (called on startup after loading settings).
pub fn set_custom_data_dir(dir: &str) {
    let mut lock = CUSTOM_DATA_DIR.write().unwrap();
    if dir.is_empty() {
        *lock = None;
    } else {
        *lock = Some(dir.to_string());
    }
}

/// Return the platform-standard data directory for CosKit.
///
/// If a custom_data_dir is configured, use that instead.
///
/// - macOS:   `~/Library/Application Support/CosKit/`
/// - Windows: `%APPDATA%/CosKit/`
/// - Linux:   `~/.local/share/CosKit/`
///
/// Falls back to `<exe_parent>/data/` if home directory cannot be determined.
pub fn data_dir() -> PathBuf {
    // Check custom override
    if let Ok(lock) = CUSTOM_DATA_DIR.read() {
        if let Some(ref custom) = *lock {
            if !custom.is_empty() {
                let p = PathBuf::from(custom);
                let _ = fs::create_dir_all(&p);
                return p;
            }
        }
    }

    default_data_dir()
}

/// The platform default data directory (ignoring custom override).
pub fn default_data_dir() -> PathBuf {
    // Prefer the runtime-resolved app data dir (set by Tauri at startup).
    // On Android/iOS this is the app-private writable directory.
    if let Ok(lock) = APP_DATA_DIR.read() {
        if let Some(ref p) = *lock {
            let _ = fs::create_dir_all(p);
            return p.clone();
        }
    }

    let dir: Option<PathBuf>;

    #[cfg(target_os = "ios")]
    {
        // iOS sandbox: ~/Documents/CosKit
        dir = std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Documents/CosKit"));
    }
    #[cfg(target_os = "android")]
    {
        // Fallback only — should be overridden by set_app_data_dir().
        dir = std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("CosKit"));
    }
    #[cfg(target_os = "macos")]
    {
        dir = dirs::home_dir().map(|h| h.join("Library/Application Support/CosKit"));
    }
    #[cfg(target_os = "windows")]
    {
        dir = std::env::var("APPDATA")
            .ok()
            .map(|a| PathBuf::from(a).join("CosKit"));
    }
    #[cfg(all(
        not(target_os = "ios"),
        not(target_os = "android"),
        not(target_os = "macos"),
        not(target_os = "windows")
    ))]
    {
        dir = dirs::home_dir().map(|h| h.join(".local/share/CosKit"));
    }

    let dir = dir.unwrap_or_else(|| {
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
    default_data_dir().join("settings.json")
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
         {{KEYWORD_HINT}}\n\
         {{REFERENCE_IMAGES_HINT}}"
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
         {{REFERENCE_IMAGES_HINT}}\n\
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
         {{REFERENCE_IMAGES_HINT}}\n\
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
         {{REFERENCE_IMAGES_HINT}}\n\
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
                if let Some(v) = saved.get("text_provider").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        settings.text_provider = v.to_string();
                    }
                }
                if let Some(v) = saved.get("image_provider").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        settings.image_provider = v.to_string();
                    }
                }
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
                if let Some(pc) = saved.get("provider_configs") {
                    if let Ok(configs) = serde_json::from_value(pc.clone()) {
                        settings.provider_configs = configs;
                    }
                }
                // Review Agent settings
                if let Some(v) = saved.get("review_enabled").and_then(|v| v.as_bool()) {
                    settings.review_enabled = v;
                }
                if let Some(v) = saved.get("review_auto_correct").and_then(|v| v.as_bool()) {
                    settings.review_auto_correct = v;
                }
                if let Some(v) = saved.get("review_threshold").and_then(|v| v.as_f64()) {
                    settings.review_threshold = v;
                }
                if let Some(v) = saved.get("review_max_retries").and_then(|v| v.as_u64()) {
                    settings.review_max_retries = v as u32;
                }
                if let Some(v) = saved.get("review_provider").and_then(|v| v.as_str()) {
                    settings.review_provider = v.to_string();
                }
                if let Some(v) = saved.get("review_model").and_then(|v| v.as_str()) {
                    settings.review_model = v.to_string();
                }
                if let Some(v) = saved.get("review_base_url").and_then(|v| v.as_str()) {
                    settings.review_base_url = v.to_string();
                }
                if let Some(v) = saved.get("review_api_key").and_then(|v| v.as_str()) {
                    settings.review_api_key = v.to_string();
                }
                if let Some(v) = saved.get("custom_data_dir").and_then(|v| v.as_str()) {
                    settings.custom_data_dir = v.to_string();
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

/// Initialize custom data dir from saved settings. Call once on startup.
pub fn init_custom_data_dir() {
    let settings = load_settings();
    set_custom_data_dir(&settings.custom_data_dir);
}

/// Migrate all session data from the current data_dir to a new directory.
/// Returns Ok(count) with number of items migrated, or Err on failure.
pub fn migrate_data_dir(new_dir: &str) -> Result<u32, String> {
    let new_path = PathBuf::from(new_dir);
    if new_dir.is_empty() {
        return Err("目标路径不能为空".to_string());
    }

    fs::create_dir_all(&new_path).map_err(|e| format!("无法创建目标目录: {e}"))?;

    let old_dir = data_dir();
    if old_dir == new_path {
        return Ok(0);
    }

    let mut count = 0u32;
    if old_dir.exists() {
        let entries = fs::read_dir(&old_dir).map_err(|e| format!("无法读取源目录: {e}"))?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip settings.json — it stays in the default location
            if name_str == "settings.json" {
                continue;
            }
            let src = entry.path();
            let dst = new_path.join(&name);
            if dst.exists() {
                continue;
            }
            if src.is_dir() {
                copy_dir_recursive(&src, &dst)?;
                count += 1;
            } else {
                fs::copy(&src, &dst).map_err(|e| format!("复制文件失败 {}: {e}", name_str))?;
                count += 1;
            }
        }
    }

    // Update the runtime override
    set_custom_data_dir(new_dir);

    Ok(count)
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败: {e}"))?;
    let entries = fs::read_dir(src).map_err(|e| format!("读取目录失败: {e}"))?;
    for entry in entries.flatten() {
        let s = entry.path();
        let d = dst.join(entry.file_name());
        if s.is_dir() {
            copy_dir_recursive(&s, &d)?;
        } else {
            fs::copy(&s, &d).map_err(|e| format!("复制失败: {e}"))?;
        }
    }
    Ok(())
}
