use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

#[derive(Serialize, Deserialize, Debug, ZeroizeOnDrop)]
pub(crate) struct Key {
    pub(crate) item: String,
    pub(crate) username: String,
    pub(crate) key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DocEntry {
    pub(crate) key: String,
    pub(crate) value: String,
}
