pub mod handlers;

use std::fmt::Debug;

use crate::error::ServerError;
use common::packet::ids::PacketId;

#[async_trait::async_trait]
pub trait PacketHandler: Send + Sync + Debug {
    async fn process(&self, packet_id: &PacketId, packet: &[u8]) -> Result<(), ServerError>;
}
