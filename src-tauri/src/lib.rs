pub mod automerge;
mod commands;
mod db;
mod error;
mod iroh;
mod model;
mod state;

use std::fs;

use ::automerge::{ReadDoc, ROOT};
use iroh::Iroh;
use state::BackgroundOutputReceiver;
use tauri::{async_runtime::RwLock, path::BaseDirectory, Manager};
use tauri_plugin_fs::FsExt;
use tauri_plugin_sql::{Migration, MigrationKind};
use tokio::sync::mpsc;

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

            let (bg_output_sender, bg_output_receiver) = mpsc::channel(1024);
            app.manage(BackgroundOutputReceiver::new(bg_output_receiver));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let (iroh, ticket, mut automerge_sync_finished) = Iroh::new().await.unwrap();
                bg_output_sender
                    .send(format!("Iroh gossip ticket: {}", ticket))
                    .await
                    .unwrap();
                handle.manage(RwLock::new(iroh));
                handle.manage(bg_output_sender.clone());

                while let Some(doc) = automerge_sync_finished.recv().await {
                    for key in doc.keys(ROOT) {
                        let (value, _) = doc.get(ROOT, &key).unwrap().unwrap();
                        bg_output_sender
                            .send(format!("{} => {}", key, value))
                            .await
                            .unwrap();
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::execute_command,
            commands::next_bg_output,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
