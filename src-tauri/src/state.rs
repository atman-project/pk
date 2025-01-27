use std::path::PathBuf;

pub(crate) struct AppState {
    pub(crate) path: PathBuf,
    pub(crate) keys: Vec<String>,
}
