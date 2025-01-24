pub mod handlers;

use crate::error::ServerError;
use common::packet::ids::PacketId;
use uuid::Uuid;

pub struct PacketData {
    client_id: Uuid,
    packet_id: PacketId,
    packet: Vec<u8>,
}

impl PacketData {
    pub fn new(client_id: Uuid, packet_id: PacketId, packet: Vec<u8>) -> Self {
        Self {
            client_id,
            packet_id,
            packet,
        }
    }
}

#[async_trait::async_trait]
pub trait PacketHandler: Send + Sync {
    async fn process(&self, data: PacketData) -> Result<(), ServerError>;
}
