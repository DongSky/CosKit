mod commands;
pub mod dotenv;
pub mod engine;
pub mod gemini_client;
pub mod image_utils;
pub mod models;
pub mod openai_client;
pub mod planner;
pub mod reviewer;
pub mod settings;
pub mod skills;
pub mod workflow;

use engine::AppState;
use gemini_client::GeminiClients;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|app| {
            // Resolve the platform-correct app data dir from Tauri.
            // On Android/iOS this is the app-private writable directory;
            // on desktop it's ~/Library/Application Support/CosKit (or equivalent).
            let resolved = app
                .path()
                .app_data_dir()
                .ok()
                .map(|p| {
                    // On desktop the path includes the bundle id; we want CosKit
                    // for backwards compat with existing data on disk.
                    #[cfg(any(target_os = "android", target_os = "ios"))]
                    {
                        p
                    }
                    #[cfg(not(any(target_os = "android", target_os = "ios")))]
                    {
                        p.parent().map(|x| x.join("CosKit")).unwrap_or(p)
                    }
                });
            if let Some(p) = resolved {
                settings::set_app_data_dir(p);
            }

            // Now that data_dir is correct, init custom override and load sessions.
            settings::init_custom_data_dir();
            let state = app.state::<AppState>();
            engine::load_all_sessions_into(state.inner());

            if let Err(e) = GeminiClients::init() {
                eprintln!("[CosKit] Warning: Gemini init failed: {e}");
                eprintln!("[CosKit] Editing will fail until GEMINI_API_KEY is set");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_image,
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
            commands::get_data_dir,
            commands::change_data_dir,
            commands::reset_data_dir,
            commands::get_workflow_status,
            commands::list_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error running CosKit");
}
