use crate::{error::ServerError, packets::PacketHandler};
use common::packet::{ids::PacketId, packet_type::PacketType, DisconnectPacket};

#[derive(Debug)]
pub struct DisconnectHandler {}

#[async_trait::async_trait]
impl PacketHandler for DisconnectHandler {
    async fn process(&self, packet_id: &PacketId, packet: &[u8]) -> Result<(), ServerError> {
        if packet_id != &PacketId::DisconnectPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = DisconnectPacket::decode(packet).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing disconnect packet: {:?}", packet);
        Ok(())
    }
}
