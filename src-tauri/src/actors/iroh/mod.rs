use std::{collections::HashMap, future::Future};

use actman::{Actor, Control, State};
use futures::stream::FuturesOrdered;
use iroh::{
    endpoint::{Connecting, Connection, RecvStream, SendStream},
    protocol::{ProtocolHandler, Router},
    NodeAddr, NodeId,
};
use tokio::sync::oneshot;

use super::automerge::{self, AutomergeActor, SyncHandle};

pub struct IrohActor {
    router: Router,
    connections: HashMap<NodeId, Connection>,

    automerge_handle: actman::Handle<AutomergeActor>,
    sync_futures: FuturesOrdered<futures_lite::future::Boxed<()>>,
}

impl IrohActor {
    pub const AUTOMERGE_ALPN: &'static [u8] = b"pk/iroh/automerge/0";

    async fn connect(&mut self, addr: NodeAddr) -> &Connection {
        let node_id = addr.node_id;
        if !self.connections.contains_key(&node_id) {
            let conn = self
                .router
                .endpoint()
                .connect(addr, Self::AUTOMERGE_ALPN)
                .await
                .unwrap();
            self.connections.entry(node_id).or_insert(conn)
        } else {
            self.connections.get(&node_id).unwrap()
        }
    }

    async fn initiate_sync(&mut self, node_addr: NodeAddr) {
        let conn = self.connect(node_addr).await;
        let (mut send, recv) = conn.open_bi().await.unwrap();

        let (tx, rx) = oneshot::channel();
        self.automerge_handle
            .send(automerge::Message::InitiateSync { sender: tx })
            .await;
        let (sync_handle, msg) = rx.await.unwrap();

        let msg_len: u64 = msg.len().try_into().unwrap();
        send.write_all(&msg_len.to_le_bytes()).await.unwrap();
        send.write_all(&msg).await.unwrap();

        let automerge_handle = self.automerge_handle.clone();
        let future = Box::pin(sync_future(sync_handle, send, recv, automerge_handle));
        self.sync_futures.push_back(future);
    }
}

async fn sync_future(
    mut sync_handle: Option<SyncHandle>,
    mut send: SendStream,
    mut recv: RecvStream,
    automerge_handle: actman::Handle<AutomergeActor>,
) {
    loop {
        let mut msg_len = [0u8; 8];
        recv.read_exact(&mut msg_len).await.unwrap();
        let msg_len = u64::from_le_bytes(msg_len);

        let mut msg = vec![0u8; msg_len.try_into().unwrap()];
        recv.read_exact(&mut msg).await.unwrap();

        let (tx, rx) = oneshot::channel();
        automerge_handle
            .send(automerge::Message::ApplySync {
                handle: sync_handle.take(),
                msg,
                sender: tx,
            })
            .await;
        let (handle, maybe_msg) = rx.await.unwrap();
        if let Some(msg) = maybe_msg {
            let msg_len: u64 = msg.len().try_into().unwrap();
            send.write_all(&msg_len.to_le_bytes()).await.unwrap();
            send.write_all(&msg).await.unwrap();
        }

        if handle.is_done() {
            return;
        } else {
            sync_handle.replace(handle);
        }
    }
}

pub enum Message {
    InitiateSync(NodeAddr),
}

#[async_trait::async_trait]
impl Actor for IrohActor {
    type Message = Message;

    async fn run(mut self, mut state: State<Self>) {
        loop {
            tokio::select! {
                Some(ctrl) = state.control_receiver.recv() => {
                    match ctrl {
                        Control::Shutdown => break,
                    }
                }
                Some(message) = state.message_receiver.recv() => {
                    match message {
                        Message::InitiateSync(node_addr) => {
                            let _ = self.connect(node_addr).await;
                        }
                    }
                }
                else => break,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct IrohAutomergeProtocol;

impl ProtocolHandler for IrohAutomergeProtocol {
    fn accept(&self, conn: Connecting) -> futures_lite::future::Boxed<anyhow::Result<()>> {
        Box::pin(async move {
            let (mut send, mut recv) = conn.await?.accept_bi().await?;
            let future = Box::pin(sync_future(None, send, recv, automerge_handle));
            Ok(())
        })
    }
}
