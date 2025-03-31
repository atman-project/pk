use tauri::async_runtime::RwLock;
use tauri_plugin_sql::{DbInstances, DbPool};
use tokio::sync::mpsc;

use crate::{error::Error, iroh::Iroh, key::Key, state::BackgroundOutputReceiver, DB_URL};

#[tauri::command]
pub async fn next_bg_output(
    bg_output_receiver: tauri::State<'_, BackgroundOutputReceiver>,
) -> Result<String, Error> {
    bg_output_receiver.recv().await
}

#[tauri::command]
pub async fn execute_command(
    db_instances: tauri::State<'_, DbInstances>,
    _iroh: tauri::State<'_, RwLock<Iroh>>,
    _bg_output_sender: tauri::State<'_, mpsc::Sender<String>>,
    command: &str,
) -> Result<String, Error> {
    let mut cmd = Command::new(command);
    match cmd.next()? {
        "l" => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).ok_or(Error::DBNotFound)?;
            Ok(serde_yaml::to_string(
                &Key::db_select_all(db).await?.iter().collect::<Vec<_>>(),
            )?)
        }
        "k" => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).ok_or(Error::DBNotFound)?;
            let item = cmd.next()?;
            let username = cmd.next()?;
            Ok(serde_yaml::to_string(&Key::db_select(db, item, username).await?).unwrap())
        }
        "i" => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).ok_or(Error::DBNotFound)?;
            let key = Key {
                item: cmd.next()?.to_string(),
                username: cmd.next()?.to_string(),
                key: cmd.next()?.to_string(),
            };
            let result = key.db_insert(db).await?;
            Ok(format!("Inserted: {:?}", result))
        }
        _ => Ok("unknown command".to_string()),
    }
}

struct Command<'a>(std::str::SplitWhitespace<'a>);

impl<'a> Command<'a> {
    fn new(data: &'a str) -> Self {
        Self(data.split_whitespace())
    }

    fn next(&mut self) -> Result<&'a str, Error> {
        self.try_next()
            .ok_or(Error::InvalidNumberOfCommandArguments)
    }

    fn try_next(&mut self) -> Option<&'a str> {
        self.0.next()
    }
}
