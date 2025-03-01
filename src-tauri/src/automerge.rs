use std::sync::Arc;

use automerge::{sync::SyncDoc, Automerge};
use iroh::{
    endpoint::{Connecting, Connection, RecvStream, SendStream},
    protocol::ProtocolHandler,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct IrohAutomergeProtocol {
    doc: Arc<Mutex<Automerge>>,
    sync_finished: mpsc::Sender<Automerge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Protocol {
    SyncMessage(Vec<u8>),
    Done,
}

impl IrohAutomergeProtocol {
    pub const ALPN: &'static [u8] = b"iroh/automerge/1";

    pub fn new(doc: Automerge, sync_finished: mpsc::Sender<Automerge>) -> Self {
        Self {
            doc: Arc::new(Mutex::new(doc)),
            sync_finished,
        }
    }

    pub async fn fork_doc(&self) -> Automerge {
        let automerge = self.doc.lock().await;
        automerge.fork()
    }

    pub async fn merge_doc(&self, doc: &mut Automerge) -> Result<(), Error> {
        let mut automerge = self.doc.lock().await;
        automerge.merge(doc)?;
        Ok(())
    }

    async fn send_msg(msg: Protocol, send: &mut SendStream) -> Result<(), Error> {
        let encoded = bincode::serialize(&msg)?;
        send.write_all(&(encoded.len() as u64).to_le_bytes())
            .await?;
        send.write_all(&encoded).await?;
        Ok(())
    }

    async fn recv_msg(recv: &mut RecvStream) -> Result<Protocol, Error> {
        let mut incoming_len = [0u8; 8];
        recv.read_exact(&mut incoming_len).await?;
        let len = u64::from_le_bytes(incoming_len);

        let mut buffer = vec![0u8; len as usize];
        recv.read_exact(&mut buffer).await?;
        Ok(bincode::deserialize(&buffer)?)
    }

    pub async fn initiate_sync(self: Arc<Self>, conn: Connection) -> Result<(), Error> {
        let (mut conn_sender, mut conn_receiver) = conn.open_bi().await?;

        let mut doc = self.fork_doc().await;
        let mut sync_state = automerge::sync::State::new();

        let mut is_local_done = false;
        loop {
            let msg = match doc.generate_sync_message(&mut sync_state) {
                Some(msg) => Protocol::SyncMessage(msg.encode()),
                None => Protocol::Done,
            };

            if !is_local_done {
                is_local_done = matches!(msg, Protocol::Done);
                Self::send_msg(msg, &mut conn_sender).await?;
            }

            let msg = Self::recv_msg(&mut conn_receiver).await?;
            let is_remote_done = matches!(msg, Protocol::Done);

            if let Protocol::SyncMessage(sync_msg) = msg {
                let sync_msg = automerge::sync::Message::decode(&sync_msg)?;
                doc.receive_sync_message(&mut sync_state, sync_msg)?;
                self.merge_doc(&mut doc).await?;
            }

            if is_remote_done && is_local_done {
                break;
            }
        }

        conn_sender.finish()?;
        Ok(())
    }

    pub async fn respond_sync(&self, conn: Connecting) -> Result<(), Error> {
        let (mut conn_sender, mut conn_receiver) = conn.await?.accept_bi().await?;

        let mut doc = self.fork_doc().await;
        let mut sync_state = automerge::sync::State::new();

        let mut is_local_done = false;
        loop {
            let msg = Self::recv_msg(&mut conn_receiver).await?;
            let is_remote_done = matches!(msg, Protocol::Done);

            if let Protocol::SyncMessage(sync_msg) = msg {
                let sync_msg = automerge::sync::Message::decode(&sync_msg)?;
                doc.receive_sync_message(&mut sync_state, sync_msg)?;
                self.merge_doc(&mut doc).await?;
            }

            let msg = match doc.generate_sync_message(&mut sync_state) {
                Some(msg) => Protocol::SyncMessage(msg.encode()),
                None => Protocol::Done,
            };

            if !is_local_done {
                is_local_done = matches!(msg, Protocol::Done);
                Self::send_msg(msg, &mut conn_sender).await?;
            }

            if is_remote_done && is_local_done {
                break;
            }
        }

        conn_sender.finish()?;
        Ok(())
    }
}

impl ProtocolHandler for IrohAutomergeProtocol {
    fn accept(&self, conn: Connecting) -> futures_lite::future::Boxed<anyhow::Result<()>> {
        let automerge = self.clone();
        Box::pin(async move {
            automerge.respond_sync(conn).await?;
            automerge
                .sync_finished
                .send(automerge.fork_doc().await)
                .await?;
            Ok(())
        })
    }
}
