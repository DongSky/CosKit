use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single raster layer of a node's document state.
///
/// A node's `layers` is a bottom-to-top stack; flattening the visible layers
/// must reproduce the node's `image_path` content. All layer rasters are
/// full-canvas RGBA PNGs at the session's original size — transparent pixels
/// reveal the layers below. Layer image files are immutable once written and
/// may be shared across nodes (a child inherits its parent's stack entries by
/// reference), so removing a layer from one node never deletes the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// "base" (bottom content) | "edit" (AI edit result). Future kinds
    /// (adjustment/text/import) must still carry a raster in `image_path`
    /// so old versions degrade gracefully.
    #[serde(default = "default_layer_kind")]
    pub kind: String,
    #[serde(default)]
    pub image_path: String,
    /// The selection mask that produced this layer (provenance, optional).
    #[serde(default)]
    pub mask_path: String,
    /// 0.0..=1.0
    #[serde(default = "default_layer_opacity")]
    pub opacity: f32,
    /// "normal" | "multiply" | "screen" | "overlay"
    #[serde(default = "default_blend_mode")]
    pub blend_mode: String,
    #[serde(default = "default_layer_visible")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
}

fn default_layer_kind() -> String {
    "edit".to_string()
}
fn default_layer_opacity() -> f32 {
    1.0
}
fn default_blend_mode() -> String {
    "normal".to_string()
}
fn default_layer_visible() -> bool {
    true
}

impl Layer {
    pub fn new(kind: &str, name: &str, image_path: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..12].to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            image_path,
            mask_path: String::new(),
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            visible: true,
            locked: false,
        }
    }

    pub fn new_base(image_path: String) -> Self {
        Self::new("base", "背景", image_path)
    }
}

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
    #[serde(default)]
    pub mask_image_path: String,
    /// Bottom-to-top layer stack. Empty = legacy flat node (image_path only);
    /// a base layer is synthesized lazily on first layer access.
    #[serde(default)]
    pub layers: Vec<Layer>,

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
            mask_image_path: String::new(),
            layers: Vec::new(),
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
    #[serde(default = "default_provider")]
    pub text_provider: String,
    #[serde(default = "default_provider")]
    pub image_provider: String,
    #[serde(default)]
    pub text_base_url: String,
    #[serde(default)]
    pub text_api_key: String,
    #[serde(default)]
    pub text_model: String,
    #[serde(default)]
    pub image_base_url: String,
    #[serde(default)]
    pub image_api_key: String,
    #[serde(default)]
    pub image_model: String,
    #[serde(default = "default_text_timeout")]
    pub text_timeout_ms: u64,
    #[serde(default = "default_image_timeout")]
    pub image_timeout_ms: u64,
    #[serde(default)]
    pub prompts: HashMap<String, String>,
    #[serde(default)]
    pub provider_configs: HashMap<String, ProviderConfig>,
    // Review Agent settings
    #[serde(default)]
    pub review_enabled: bool,
    #[serde(default)]
    pub review_auto_correct: bool,
    #[serde(default = "default_review_threshold")]
    pub review_threshold: f64,
    #[serde(default = "default_review_max_retries")]
    pub review_max_retries: u32,
    #[serde(default)]
    pub review_provider: String,
    #[serde(default)]
    pub review_model: String,
    #[serde(default)]
    pub review_base_url: String,
    #[serde(default)]
    pub review_api_key: String,
    #[serde(default)]
    pub custom_data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_provider() -> String {
    "gemini".to_string()
}

fn default_text_timeout() -> u64 {
    180000
}
fn default_image_timeout() -> u64 {
    300000
}
fn default_review_threshold() -> f64 {
    7.0
}
fn default_review_max_retries() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineModules {
    #[serde(default = "default_true")]
    pub retouch: bool,
    #[serde(default)]
    pub background: bool,
    #[serde(default)]
    pub effects: bool,
    #[serde(default = "default_true")]
    pub agent_mode: bool,
    #[serde(default = "default_true")]
    pub save_intermediates: bool,
    #[serde(default)]
    pub combined_mode: bool,
    #[serde(default)]
    pub review_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PipelineModules {
    fn default() -> Self {
        Self {
            retouch: true,
            background: false,
            effects: false,
            agent_mode: true,
            save_intermediates: true,
            combined_mode: false,
            review_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceImage {
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub description: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            text_provider: default_provider(),
            image_provider: default_provider(),
            text_base_url: String::new(),
            text_api_key: String::new(),
            text_model: String::new(),
            image_base_url: String::new(),
            image_api_key: String::new(),
            image_model: String::new(),
            text_timeout_ms: 180000,
            image_timeout_ms: 300000,
            prompts: crate::settings::default_prompts(),
            provider_configs: HashMap::new(),
            review_enabled: false,
            review_auto_correct: false,
            review_threshold: default_review_threshold(),
            review_max_retries: default_review_max_retries(),
            review_provider: String::new(),
            review_model: String::new(),
            review_base_url: String::new(),
            review_api_key: String::new(),
            custom_data_dir: String::new(),
        }
    }
}
