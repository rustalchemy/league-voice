use crate::error::ClientError;
use tokio::sync::mpsc::{Receiver, Sender};

pub mod codec;
pub mod cpal;

#[async_trait::async_trait]
pub trait AudioHandler: Send + Sync {
    async fn retrieve(
        &self,
        input: Sender<Vec<u8>>,
        output: Receiver<Vec<u8>>,
    ) -> Result<(), ClientError>;
}
