use actman::{Actor, Control, State};
use automerge::{sync::SyncDoc, transaction::Transactable, Automerge};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

pub struct AutomergeActor {
    doc: Automerge,
}

impl AutomergeActor {
    fn update_doc(&mut self, key: String, value: String) {
        let mut doc = self.doc.fork();
        let mut tx = doc.transaction();
        tx.put(automerge::ROOT, key, value).unwrap();
        tx.commit();
        self.doc.merge(&mut doc).unwrap();
    }

    fn initiate_sync(&mut self) -> (SyncHandle, Vec<u8>) {
        let doc = self.doc.fork();
        let mut sync_state = automerge::sync::State::new();
        let protocol = match doc.generate_sync_message(&mut sync_state) {
            Some(msg) => Protocol::SyncMessage(msg.encode()),
            None => Protocol::Done,
        };
        (
            SyncHandle {
                doc,
                sync_state,
                is_local_done: matches!(protocol, Protocol::Done),
                is_remote_done: false,
            },
            bincode::serialize(&protocol).unwrap(),
        )
    }

    fn apply_sync(
        &mut self,
        handle: Option<SyncHandle>,
        msg: Vec<u8>,
    ) -> (SyncHandle, Option<Vec<u8>>) {
        let mut sync_handle = match handle {
            Some(handle) => handle,
            None => SyncHandle {
                doc: self.doc.fork(),
                sync_state: automerge::sync::State::new(),
                is_local_done: false,
                is_remote_done: false,
            },
        };

        let protocol: Protocol = bincode::deserialize(&msg).unwrap();
        match protocol {
            Protocol::SyncMessage(msg) => {
                let msg = automerge::sync::Message::decode(&msg).unwrap();
                sync_handle
                    .doc
                    .receive_sync_message(&mut sync_handle.sync_state, msg)
                    .unwrap();
                self.doc.merge(&mut sync_handle.doc).unwrap();
            }
            Protocol::Done => {
                sync_handle.is_remote_done = true;
            }
        }

        if sync_handle.is_local_done {
            (sync_handle, None)
        } else {
            let protocol = match sync_handle
                .doc
                .generate_sync_message(&mut sync_handle.sync_state)
            {
                Some(msg) => Protocol::SyncMessage(msg.encode()),
                None => Protocol::Done,
            };
            sync_handle.is_local_done = matches!(protocol, Protocol::Done);
            (sync_handle, Some(bincode::serialize(&protocol).unwrap()))
        }
    }
}

#[derive(Debug)]
pub struct SyncHandle {
    doc: Automerge,
    sync_state: automerge::sync::State,
    is_local_done: bool,
    is_remote_done: bool,
}

impl SyncHandle {
    pub fn is_done(&self) -> bool {
        self.is_local_done && self.is_remote_done
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Protocol {
    SyncMessage(Vec<u8>),
    Done,
}

pub enum Message {
    UpdateDoc {
        key: String,
        value: String,
    },
    InitiateSync {
        sender: oneshot::Sender<(SyncHandle, Vec<u8>)>,
    },
    ApplySync {
        handle: Option<SyncHandle>,
        msg: Vec<u8>,
        sender: oneshot::Sender<(SyncHandle, Option<Vec<u8>>)>,
    },
}

#[async_trait::async_trait]
impl Actor for AutomergeActor {
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
                        Message::UpdateDoc{ key, value } => {
                            self.update_doc(key, value);
                        }
                        Message::InitiateSync { sender } => {
                            sender.send(self.initiate_sync()).unwrap();
                        }
                        Message::ApplySync { handle, msg, sender } => {
                            sender.send(self.apply_sync(handle, msg)).unwrap();
                        }
                    }
                }
                else => break,
            }
        }
    }
}
