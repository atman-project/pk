mod commands;
mod db;
mod error;
mod iroh;
mod key;
mod state;

use std::fs;

use error::Error;
use futures_lite::StreamExt;
use iroh::Iroh;
use iroh_gossip::net::GossipReceiver;
use state::BackgroundOutputReceiver;
use tauri::{async_runtime::RwLock, path::BaseDirectory, App, Manager};
use tauri_plugin_cli::CliExt;
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
        .plugin(tauri_plugin_cli::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(DB_URL, db_migrations)
                .build(),
        )
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_cli::init())
        .setup(|app| {
            let args = parse_cli_args(app).unwrap();

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
                let (iroh, gossip_receiver) = Iroh::new(args.ticket).await.unwrap();
                handle.manage(RwLock::new(iroh));
                handle_gossip_events(gossip_receiver, bg_output_sender)
                    .await
                    .unwrap();
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

struct Args {
    ticket: Option<String>,
}

fn parse_cli_args(app: &App) -> Result<Args, Error> {
    let matches = app
        .cli()
        .matches()
        .map_err(|e| Error::Argument(e.to_string()))?;
    let ticket = matches.args.get("ticket").and_then(|arg| match &arg.value {
        serde_json::Value::String(ticket) => Some(ticket.clone()),
        serde_json::Value::Null => None,
        _ => None,
    });
    Ok(Args { ticket })
}

async fn handle_gossip_events(
    mut receiver: GossipReceiver,
    bg_output_sender: mpsc::Sender<String>,
) -> Result<(), Error> {
    while let Some(event) = receiver
        .try_next()
        .await
        .map_err(|e| Error::Gossip(format!("gossip receiver error: {e:?}")))?
    {
        let output = match event {
            iroh_gossip::net::Event::Gossip(event) => match event {
                iroh_gossip::net::GossipEvent::Received(message) => {
                    format!(
                        "Gossip: {:?}: {}",
                        message,
                        String::from_utf8_lossy(&message.content)
                    )
                }
                _ => format!("Gossip: {:?}", event),
            },
            iroh_gossip::net::Event::Lagged => "Gossip: Lagged".to_string(),
        };
        bg_output_sender
            .send(output)
            .await
            .map_err(|e| Error::Channel(e.to_string()))?;
    }

    Err(Error::Gossip("gossip receiver returned None".to_string()))
}
