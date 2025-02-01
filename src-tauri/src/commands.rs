use tauri::async_runtime::RwLock;
use tauri_plugin_sql::{DbInstances, DbPool};

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
    iroh: tauri::State<'_, RwLock<Iroh>>,
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
        "b" => {
            let msg = cmd.next()?.to_owned();
            let lock = iroh.read().await;
            lock.gossip_sender
                .broadcast(msg.clone().into())
                .await
                .map_err(|e| Error::Gossip(e.to_string()))?;
            Ok(format!("Broadcasted: {msg}").to_string())
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
        self.0.next().ok_or(Error::InvalidNumberOfCommandArguments)
    }
}
