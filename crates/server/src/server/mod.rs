use crate::error::ServerError;
use common::packet::Packet;
use std::{borrow::Cow, sync::Arc};

pub mod tokio;

pub trait Server: Send + Sync {
    type Handlers;

    async fn run(&mut self, addr: Cow<'_, str>) -> Result<(), ServerError>;
    async fn process_packet(
        handlers: Arc<Self::Handlers>,
        packet: Packet,
    ) -> Result<(), ServerError>;
}
