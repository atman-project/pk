use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    sync::Arc,
};

use iroh::{
    discovery::local_swarm_discovery::LocalSwarmDiscovery, protocol::Router, NodeAddr, SecretKey,
};
use iroh_blobs::{net_protocol::Blobs, util::local_pool::LocalPool};
use iroh_gossip::{
    net::{Gossip, GossipReceiver, GossipSender},
    proto::TopicId,
};
use serde::{Deserialize, Serialize};

pub(crate) struct Iroh {
    router: Router,
    gossip: Gossip,
    gossip_topic_id: TopicId,
    _blobs_local_pool: Arc<LocalPool>,
    pub(crate) gossip_sender: Option<GossipSender>,
}

impl Iroh {
    pub async fn new() -> anyhow::Result<(Self, String)> {
        let key = SecretKey::generate(rand::rngs::OsRng);
        let id = key.public();

        let builder = iroh::Endpoint::builder()
            .secret_key(key)
            .relay_mode(iroh::RelayMode::Default)
            .discovery_n0()
            .discovery(Box::new(LocalSwarmDiscovery::new(id)?));

        let endpoint = builder.bind().await?;
        println!(
            "Listening on: {}: {:?}",
            endpoint.node_id(),
            endpoint.node_addr().await.unwrap()
        );

        let blobs_local_pool = LocalPool::default();
        let blobs = Blobs::memory().build(blobs_local_pool.handle(), &endpoint);

        let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, blobs.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
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
                _blobs_local_pool: Arc::new(blobs_local_pool),
                gossip_sender: None,
            },
            ticket.to_string(),
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
