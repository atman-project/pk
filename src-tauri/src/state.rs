use tokio::sync::{mpsc, Mutex};

use crate::error::Error;

pub(crate) struct BackgroundOutputReceiver(Mutex<mpsc::Receiver<String>>);

impl BackgroundOutputReceiver {
    pub(crate) fn new(rx: mpsc::Receiver<String>) -> Self {
        Self(Mutex::new(rx))
    }

    pub(crate) async fn recv(&self) -> Result<String, Error> {
        self.0.lock().await.recv().await.ok_or(Error::NoOutput)
    }
}
