mod commands;
pub mod dotenv;
pub mod engine;
pub mod gemini_client;
pub mod image_utils;
pub mod models;
pub mod settings;

use engine::AppState;
use gemini_client::GeminiClients;

pub fn run() {
    let state = AppState::new();
    engine::load_all_sessions_into(&state);

    // Initialize Gemini clients (non-fatal on failure)
    if let Err(e) = GeminiClients::init() {
        eprintln!("[CosKit] Warning: Gemini init failed: {e}");
        eprintln!("[CosKit] Editing will fail until GEMINI_API_KEY is set");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::create_session,
            commands::get_session,
            commands::list_sessions,
            commands::delete_session,
            commands::submit_edit,
            commands::get_node_status,
            commands::navigate_branch,
            commands::goto_node,
            commands::get_image,
            commands::export_image,
            commands::get_settings,
            commands::save_settings,
            commands::get_default_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error running CosKit");
}
