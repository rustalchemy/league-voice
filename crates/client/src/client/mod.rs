use crate::{
    audio::{AudioHandler, DeviceHandler},
    error::ClientError,
};
use std::borrow::Cow;

pub mod tokio;

#[async_trait::async_trait]
pub trait Client<A: AudioHandler, D: DeviceHandler>: Send + Sync + Sized {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, ClientError>;
    async fn run(mut self) -> Result<(), ClientError>;

    fn audio_handler(&self) -> &A;
    fn audio_handler_mut(&mut self) -> &mut A;

    fn device_handler(&self) -> &D;
    fn device_handler_mut(&mut self) -> &mut D;

    async fn stop(&self) -> Result<(), ClientError>;
}
