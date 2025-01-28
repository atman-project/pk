use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

#[derive(Serialize, Deserialize, Debug, ZeroizeOnDrop)]
pub(crate) struct Key {
    pub(crate) item: String,
    pub(crate) username: String,
    pub(crate) key: String,
}
