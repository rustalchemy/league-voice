use crate::{audio::AudioHandler, error::ClientError};
use std::borrow::Cow;

pub mod tokio;

pub trait Client<A: AudioHandler>: Sized {
    async fn connect(addr: Cow<'_, str>, audio_handler: A) -> Result<Self, ClientError>;
    async fn run(self) -> Result<(), ClientError>;
}
