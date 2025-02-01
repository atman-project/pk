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
    #[error("Argument error: {0}")]
    Argument(String),
    #[error("Channel: {0}")]
    Channel(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
