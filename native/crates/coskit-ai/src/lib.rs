//! CosKit's existing provider, planner, reviewer and workflow code, shared without a webview.
#[path = "../../../../src-tauri/src/beauty_filter.rs"]
pub mod beauty_filter;
#[path = "../../../../src-tauri/src/dotenv.rs"]
pub mod dotenv;
// Keep the original workflow API signatures shared with the legacy frontend.
#[allow(clippy::too_many_arguments)]
#[path = "../../../../src-tauri/src/engine.rs"]
pub mod engine;
#[path = "../../../../src-tauri/src/gemini_client.rs"]
pub mod gemini_client;
#[path = "../../../../src-tauri/src/image_utils.rs"]
pub mod image_utils;
#[path = "../../../../src-tauri/src/models.rs"]
pub mod models;
#[allow(clippy::too_many_arguments)]
#[path = "../../../../src-tauri/src/openai_client.rs"]
pub mod openai_client;
#[path = "../../../../src-tauri/src/planner.rs"]
pub mod planner;
#[path = "../../../../src-tauri/src/reviewer.rs"]
pub mod reviewer;
#[path = "../../../../src-tauri/src/settings.rs"]
pub mod settings;
#[path = "../../../../src-tauri/src/skills.rs"]
pub mod skills;
#[allow(clippy::too_many_arguments)]
#[path = "../../../../src-tauri/src/workflow.rs"]
pub mod workflow;

impl Default for engine::AppState {
    fn default() -> Self {
        Self::new()
    }
}
