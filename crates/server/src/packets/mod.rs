pub mod handlers;

use crate::error::ServerError;
use common::packet::ids::PacketId;
use uuid::Uuid;

pub struct PacketData {
    client_id: Uuid,
    packet_id: PacketId,
    data: Vec<u8>, // This is the raw packet data,
}

impl PacketData {
    pub fn new(client_id: Uuid, packet_id: PacketId, packet: Vec<u8>) -> Self {
        Self {
            client_id,
            packet_id,
            data: packet,
        }
    }
}

#[async_trait::async_trait]
pub trait PacketHandler: Send + Sync {
    async fn process(&self, data: PacketData) -> Result<(), ServerError>;
}
