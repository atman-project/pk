// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn execute_command(command: &str) -> String {
    format!("Command:{} executed", command)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
