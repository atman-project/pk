mod commands;
mod state;

use std::{fs, sync::Mutex};

use state::AppState;
use tauri::{path::BaseDirectory, Manager};
use tauri_plugin_fs::FsExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let base_path = app.path().resolve("", BaseDirectory::AppData).unwrap();
            app.fs_scope()
                .allow_directory(base_path.clone(), true)
                .unwrap();
            fs::create_dir_all(base_path.clone()).unwrap();

            let file_path = base_path.join("keys");
            if !fs::exists(&file_path).unwrap() {
                fs::write(&file_path, "k1\nk2").unwrap();
            }

            app.manage(Mutex::new(AppState {
                path: file_path,
                keys: vec!["a".to_string(), "b".to_string()],
            }));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
