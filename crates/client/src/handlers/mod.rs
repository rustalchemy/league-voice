use crate::error::ClientError;
use common::packet::Packet;

pub mod audio;
pub mod connect;
pub mod disconnect;

#[async_trait::async_trait]
pub trait PacketHandler {
    async fn handle_packet(&self, packet: Packet) -> Result<(), ClientError>;
}
