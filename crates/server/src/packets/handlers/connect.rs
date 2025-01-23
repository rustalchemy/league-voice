use crate::{error::ServerError, packets::PacketHandler};
use common::packet::{ids::PacketId, packet_type::PacketType, ConnectPacket};

#[derive(Debug)]
pub struct ConnectHandler {}

#[async_trait::async_trait]
impl PacketHandler for ConnectHandler {
    async fn process(&self, packet_id: &PacketId, packet: &[u8]) -> Result<(), ServerError> {
        if packet_id != &PacketId::ConnectPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = ConnectPacket::decode(packet).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing connect packet: {:?}", packet);
        Ok(())
    }
}
