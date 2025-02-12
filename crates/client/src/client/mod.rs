use crate::{
    audio::{DeviceHandler, SoundProcessor},
    error::ClientError,
};
use std::borrow::Cow;

pub mod tokio;

#[async_trait::async_trait]
pub trait Client<S: SoundProcessor, D: DeviceHandler>: Send + Sync + Sized {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, ClientError>;
    async fn run(&mut self) -> Result<(), ClientError>;

    fn device_handler(&self) -> &D;
    fn device_handler_mut(&mut self) -> &mut D;

    async fn stop(&mut self) -> Result<(), ClientError>;

    async fn is_running(&self) -> bool;
}
