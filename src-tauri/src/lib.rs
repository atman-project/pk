use std::sync::Mutex;

use tauri::Manager;

struct AppState {
    keys: Vec<String>,
}

#[tauri::command]
fn execute_command(state: tauri::State<Mutex<AppState>>, command: &str) -> String {
    match command {
        "ks" => {
            let state = state.lock().unwrap();
            state.keys.join(", ")
        }
        _ => "unknown command".to_string(),
    }
}

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
        .invoke_handler(tauri::generate_handler![execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
