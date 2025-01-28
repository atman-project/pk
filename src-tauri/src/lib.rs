mod commands;
mod state;

use std::{fs, sync::Mutex};

use state::AppState;
use tauri::{path::BaseDirectory, Manager};
use tauri_plugin_fs::FsExt;
use tauri_plugin_sql::{Migration, MigrationKind};

pub(crate) const DB_URL: &str = "sqlite:pk.db";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_migrations = vec![Migration {
        version: 1,
        description: "create_initial_tables",
        sql: r#"
            CREATE TABLE IF NOT EXISTS keys (
                item TEXT NOT NULL,
                username TEXT NOT NULL,
                key TEXT,
                created_at INTEGER,
                updated_at INTEGER,
                PRIMARY KEY (item, username)
            );"#,
        kind: MigrationKind::Up,
    }];

    tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(DB_URL, db_migrations)
                .build(),
        )
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
