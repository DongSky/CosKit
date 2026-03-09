use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditNode {
    pub id: String,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub image_path: String,
    #[serde(default)]
    pub thumbnail_path: String,
    #[serde(default)]
    pub note: String,
    #[serde(default = "default_status")]
    pub status: String,
    pub error_msg: Option<String>,
    #[serde(default = "now_timestamp")]
    pub created_at: f64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    // Transient progress fields — not persisted
    #[serde(skip)]
    pub progress_step: u32,
    #[serde(skip)]
    pub progress_total: u32,
    #[serde(skip)]
    pub progress_msg: String,
}

fn default_status() -> String {
    "pending".to_string()
}

fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

impl EditNode {
    pub fn new(id: String, parent_id: Option<String>) -> Self {
        Self {
            id,
            parent_id,
            children: Vec::new(),
            prompt: String::new(),
            image_path: String::new(),
            thumbnail_path: String::new(),
            note: String::new(),
            status: "pending".to_string(),
            error_msg: None,
            created_at: now_timestamp(),
            metadata: HashMap::new(),
            progress_step: 0,
            progress_total: 0,
            progress_msg: String::new(),
        }
    }

    /// Serialize for API output (excludes transient fields via serde skip)
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub root_id: String,
    #[serde(default)]
    pub nodes: HashMap<String, EditNode>,
    #[serde(default)]
    pub original_size: (u32, u32),
    #[serde(default)]
    pub active_path: Vec<String>,
    #[serde(default = "now_timestamp")]
    pub created_at: f64,
}

impl Session {
    pub fn new(id: String, root_id: String, original_size: (u32, u32)) -> Self {
        Self {
            id,
            root_id,
            nodes: HashMap::new(),
            original_size,
            active_path: Vec::new(),
            created_at: now_timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub text_base_url: String,
    #[serde(default)]
    pub text_api_key: String,
    #[serde(default)]
    pub image_base_url: String,
    #[serde(default)]
    pub image_api_key: String,
    #[serde(default = "default_text_timeout")]
    pub text_timeout_ms: u64,
    #[serde(default = "default_image_timeout")]
    pub image_timeout_ms: u64,
    #[serde(default)]
    pub prompts: HashMap<String, String>,
}

fn default_text_timeout() -> u64 {
    180000
}
fn default_image_timeout() -> u64 {
    300000
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            text_base_url: String::new(),
            text_api_key: String::new(),
            image_base_url: String::new(),
            image_api_key: String::new(),
            text_timeout_ms: 180000,
            image_timeout_ms: 300000,
            prompts: crate::settings::default_prompts(),
        }
    }
}
