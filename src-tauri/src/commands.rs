use std::sync::Mutex;

use crate::state::AppState;

#[tauri::command]
pub fn execute_command(state: tauri::State<Mutex<AppState>>, command: &str) -> String {
    match command {
        "ks" => {
            let state = state.lock().unwrap();
            state.keys.join(", ")
        }
        _ => "unknown command".to_string(),
    }
}
