use tauri_plugin_sql::{DbInstances, DbPool};

use crate::{error::Error, key::Key, DB_URL};

#[tauri::command]
pub async fn execute_command(
    db_instances: tauri::State<'_, DbInstances>,
    command: &str,
) -> Result<String, Error> {
    let mut iter = command.split_whitespace();
    match iter.next() {
        Some("l") => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).ok_or(Error::DBNotFound)?;
            Ok(serde_yaml::to_string(
                &Key::db_select_all(db).await?.iter().collect::<Vec<_>>(),
            )?)
        }
        Some("k") => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).ok_or(Error::DBNotFound)?;
            let item = iter.next().ok_or(Error::InvalidNumberOfCommandArguments)?;
            let username = iter.next().ok_or(Error::InvalidNumberOfCommandArguments)?;
            Ok(serde_yaml::to_string(&Key::db_select(db, item, username).await?).unwrap())
        }
        Some("i") => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).ok_or(Error::DBNotFound)?;
            let key = Key {
                item: iter
                    .next()
                    .ok_or(Error::InvalidNumberOfCommandArguments)?
                    .to_string(),
                username: iter
                    .next()
                    .ok_or(Error::InvalidNumberOfCommandArguments)?
                    .to_string(),
                key: iter
                    .next()
                    .ok_or(Error::InvalidNumberOfCommandArguments)?
                    .to_string(),
            };
            let result = key.db_insert(db).await?;
            Ok(format!("Inserted: {:?}", result))
        }
        Some(_) | None => Ok("unknown command".to_string()),
    }
}
