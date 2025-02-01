use crate::{audio::AudioHandler, error::ClientError};
use std::{borrow::Cow, sync::Arc};

pub mod tokio;

#[async_trait::async_trait]
pub trait Client<A: AudioHandler>: Send + Sync + Sized {
    async fn connect(addr: Cow<'_, str>, audio_handler: A) -> Result<Self, ClientError>;
    async fn run(self) -> Result<(), ClientError>;

    fn audio_handler(&self) -> Arc<A>;
}
