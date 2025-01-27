mod commands;
mod state;

use std::sync::Mutex;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState {
                keys: vec!["a".to_string(), "b".to_string()],
            }));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
