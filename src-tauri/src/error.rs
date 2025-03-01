use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("DB not found")]
    DBNotFound,
    #[error("Invalid number of command arguments")]
    InvalidNumberOfCommandArguments,
    #[error("DB error: {0}")]
    DB(#[from] sqlx::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Gossip error: {0}")]
    Gossip(String),
    #[error("No output")]
    NoOutput,
    #[error("Channel: {0}")]
    Channel(String),
    #[error("Iroh error: {0}")]
    Iroh(#[from] anyhow::Error),
    #[error("Iroh connection error: {0}")]
    IrohConnection(#[from] iroh::endpoint::ConnectionError),
    #[error("Iroh write error: {0}")]
    IrohWriteError(#[from] iroh::endpoint::WriteError),
    #[error("Iroh read exact error: {0}")]
    IrohReadExactError(#[from] iroh::endpoint::ReadExactError),
    #[error("Iroh close stream error: {0}")]
    IrohClosedStream(#[from] iroh::endpoint::ClosedStream),
    #[error("Automerge error: {0}")]
    Automerge(#[from] automerge::AutomergeError),
    #[error("Automerge read message error")]
    AutomergeReadMessage(#[from] automerge::sync::ReadMessageError),
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
