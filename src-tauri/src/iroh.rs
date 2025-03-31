use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use iroh::{protocol::Router, NodeAddr, SecretKey};
use serde::{Deserialize, Serialize};

pub(crate) struct Iroh {
    router: Router,
}

impl Iroh {
    pub async fn new() -> anyhow::Result<(Self, String)> {
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

        let router = Router::builder(endpoint.clone()).spawn().await?;

        let ticket = Ticket {
            nodes: vec![endpoint.node_addr().await.unwrap()],
        };
        println!("Ticket: {}", ticket);

        Ok((Self { router }, ticket.to_string()))
    }

    #[allow(dead_code)]
    pub(crate) async fn shutdown(self) -> anyhow::Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Ticket {
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
