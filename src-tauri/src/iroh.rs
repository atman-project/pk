use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use automerge::{transaction::Transactable, Automerge};
use iroh::{protocol::Router, NodeAddr, NodeId, SecretKey};
use iroh_gossip::{
    net::{Gossip, GossipReceiver, GossipSender},
    proto::TopicId,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::automerge::IrohAutomergeProtocol;

pub(crate) struct Iroh {
    router: Router,
    gossip: Gossip,
    gossip_topic_id: TopicId,
    pub(crate) gossip_sender: Option<GossipSender>,
    automerge: IrohAutomergeProtocol,
}

impl Iroh {
    pub async fn new() -> anyhow::Result<(Self, String, mpsc::Receiver<Automerge>)> {
        let key = SecretKey::generate(rand::rngs::OsRng);

        let builder = iroh::Endpoint::builder()
            .secret_key(key)
            .relay_mode(iroh::RelayMode::Default)
            .discovery_n0();

        let endpoint = builder.bind().await?;
        println!(
            "Listening on: {}: {:?}",
            endpoint.node_id(),
            endpoint.node_addr().await.unwrap()
        );

        let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

        let (automerge_sync_sender, automerge_sync_finished) = mpsc::channel(10);
        let automerge = IrohAutomergeProtocol::new(Automerge::new(), automerge_sync_sender);

        let router = Router::builder(endpoint.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .accept(IrohAutomergeProtocol::ALPN, automerge.clone())
            .spawn()
            .await?;

        let gossip_topic_id = TopicId::from_bytes([0u8; 32]);
        let ticket = Ticket {
            topic: gossip_topic_id,
            nodes: vec![endpoint.node_addr().await.unwrap()],
        };
        println!("Ticket: {}", ticket);

        Ok((
            Self {
                router,
                gossip,
                gossip_topic_id,
                gossip_sender: None,
                automerge,
            },
            ticket.to_string(),
            automerge_sync_finished,
        ))
    }

    #[allow(dead_code)]
    pub(crate) async fn shutdown(self) -> anyhow::Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }

    pub(crate) async fn gossip_subscribe(
        &mut self,
        ticket: Option<String>,
    ) -> anyhow::Result<GossipReceiver> {
        if self.gossip_sender.is_some() {
            return Err(anyhow::anyhow!("Already subscribed"));
        }

        let gossip_topic = match ticket {
            Some(ticket) => {
                let ticket = Ticket::from_str(&ticket)?;
                self.gossip
                    .subscribe_and_join(
                        self.gossip_topic_id,
                        ticket
                            .nodes
                            .into_iter()
                            .map(|node_addr| node_addr.node_id)
                            .collect(),
                    )
                    .await?
            }
            None => self.gossip.subscribe(self.gossip_topic_id, vec![])?,
        };
        let (sender, receiver) = gossip_topic.split();
        self.gossip_sender.replace(sender);
        Ok(receiver)
    }

    pub(crate) async fn update_doc(&mut self, key: String, value: String) -> anyhow::Result<()> {
        let mut doc = self.automerge.fork_doc().await;
        let mut tx = doc.transaction();
        tx.put(automerge::ROOT, key, value)?;
        tx.commit();
        self.automerge.merge_doc(&mut doc).await?;
        Ok(())
    }

    pub(crate) async fn doc_sync(&self, node_id: NodeId) -> anyhow::Result<()> {
        let node_addr = NodeAddr::new(node_id);
        let conn = self
            .router
            .endpoint()
            .connect(node_addr, IrohAutomergeProtocol::ALPN)
            .await?;
        self.automerge.initiate_sync(conn).await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Ticket {
    topic: TopicId,
    nodes: Vec<NodeAddr>,
}

impl Ticket {
    /// Deserialize from a slice of bytes to a Ticket.
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    /// Serialize from a `Ticket` to a `Vec` of bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

// The `Display` trait allows us to use the `to_string`
// method on `Ticket`.
impl Display for Ticket {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

// The `FromStr` trait allows us to turn a `str` into
// a `Ticket`
impl FromStr for Ticket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD.decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}
