use std::sync::Mutex;

use crate::state::AppState;

#[tauri::command]
pub fn execute_command(state: tauri::State<Mutex<AppState>>, command: &str) -> String {
    match command {
        "ks" => {
            let state = state.lock().unwrap();
            state.keys.join(", ")
        }
        "p" => {
            let state = state.lock().unwrap();
            state.path.to_string_lossy().to_string()
        }
        "f" => {
            let state = state.lock().unwrap();
            let ret = std::fs::read_to_string(&state.path).unwrap();
            std::fs::write(&state.path, format!("{}\n{}", ret, "k3")).unwrap();
            ret
        }
        _ => "unknown command".to_string(),
    }
}
