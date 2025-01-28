use std::{sync::Mutex, time::SystemTime};

use sqlx::{Executor, Row};
use tauri_plugin_sql::{DbInstances, DbPool};

use crate::{state::AppState, DB_URL};

#[tauri::command]
pub async fn execute_command(
    app_state: tauri::State<'_, Mutex<AppState>>,
    db_instances: tauri::State<'_, DbInstances>,
    command: &str,
) -> Result<String, ()> {
    match command {
        "ks" => {
            let state = app_state.lock().unwrap();
            Ok(state.keys.join(", "))
        }
        "p" => {
            let state = app_state.lock().unwrap();
            Ok(state.path.to_string_lossy().to_string())
        }
        "f" => {
            let state = app_state.lock().unwrap();
            let ret = std::fs::read_to_string(&state.path).unwrap();
            std::fs::write(&state.path, format!("{}\n{}", ret, "k3")).unwrap();
            Ok(ret)
        }
        "d" => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).unwrap();

            let query = sqlx::query("SELECT * FROM keys");
            let rows = db.fetch_all(query).await.unwrap();
            let mut results = Vec::new();
            rows.iter().for_each(|row| {
                let item: &str = row.try_get("item").unwrap();
                let username: &str = row.try_get("username").unwrap();
                let key: &str = row.try_get("key").unwrap();
                let created_at: i64 = row.try_get("created_at").unwrap();
                let updated_at: i64 = row.try_get("updated_at").unwrap();
                results.push(format!(
                    "item: {}, username: {}, key: {}, created_at: {}, updated_at: {}",
                    item, username, key, created_at, updated_at
                ));
            });
            Ok(results.join("\n"))
        }
        "di" => {
            let lock = db_instances.0.read().await;
            let DbPool::Sqlite(db) = lock.get(DB_URL).unwrap();

            let ts = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let query = sqlx::query(
                r#"
INSERT INTO keys ( item, username, key, created_at, updated_at )
VALUES ( $1, $2, $3, $4, $5 )
        "#,
            )
            .bind("item!")
            .bind("username!")
            .bind("key!")
            .bind(ts as i64)
            .bind(ts as i64);
            let result = db.execute(query).await.unwrap();
            println!("result: {:?}", result);

            Ok("inserted".to_string())
        }
        _ => Ok("unknown command".to_string()),
    }
}
