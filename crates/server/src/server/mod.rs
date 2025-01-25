pub mod client;
pub mod tokio;

use crate::error::ServerError;
use client::Clients;
use common::packet::Packet;
use std::{borrow::Cow, sync::Arc};
use uuid::Uuid;

pub(crate) trait Server: Send + Sync {
    type Handlers;

    async fn run(&mut self, addr: Cow<'_, str>) -> Result<(), ServerError>;
    async fn process_packet(
        client_id: Uuid,
        handlers: Arc<Self::Handlers>,
        packet: Packet,
    ) -> Result<(), ServerError>;
    fn clients(&self) -> Arc<Clients>;
}
