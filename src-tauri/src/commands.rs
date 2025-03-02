use std::str::FromStr;

use futures_lite::StreamExt;
use iroh::NodeId;
use iroh_gossip::net::GossipReceiver;
use tauri::async_runtime::RwLock;
use tauri_plugin_sql::{DbInstances, DbPool};
use tokio::sync::mpsc;

use crate::{error::Error, iroh::Iroh, model::Key, state::BackgroundOutputReceiver, DB_URL};

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
    bg_output_sender: tauri::State<'_, mpsc::Sender<String>>,
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
        "g" => {
            let ticket = cmd.try_next().map(|s| s.to_string());
            let mut lock = iroh.write().await;
            let gossip_receiver = lock.gossip_subscribe(ticket.clone()).await?;
            let bg_output_sender = bg_output_sender.inner().clone();
            tokio::spawn(async move {
                handle_gossip_events(gossip_receiver, bg_output_sender)
                    .await
                    .unwrap();
            });
            Ok(format!("Gossip joined with ticket: {:?}", ticket).to_string())
        }
        "b" => {
            let msg = cmd.next()?.to_owned();
            let lock = iroh.read().await;
            if let Some(gossip_sender) = &lock.gossip_sender {
                gossip_sender
                    .broadcast(msg.clone().into())
                    .await
                    .map_err(|e| Error::Gossip(e.to_string()))?;
                Ok(format!("Broadcasted: {msg}").to_string())
            } else {
                Err(Error::Gossip("Not subscribed yet".to_string()))
            }
        }
        "dp" => {
            let mut lock = iroh.write().await;
            lock.update_doc(cmd.next()?.to_string(), cmd.next()?.to_string())
                .await?;
            Ok("Document updated".to_string())
        }
        "ds" => {
            let node_id = NodeId::from_str(cmd.next()?).unwrap();
            let mut lock = iroh.write().await;
            lock.doc_sync(node_id).await?;
            Ok("Document synced".to_string())
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
