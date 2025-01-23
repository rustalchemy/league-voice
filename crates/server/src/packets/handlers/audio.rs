use crate::{error::ServerError, packets::PacketHandler};
use common::packet::{ids::PacketId, packet_type::PacketType, AudioPacket};

#[derive(Debug)]
pub struct AudioHandler;

#[async_trait::async_trait]
impl PacketHandler for AudioHandler {
    async fn process(&self, packet_id: &PacketId, packet: &[u8]) -> Result<(), ServerError> {
        if packet_id != &PacketId::AudioPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = AudioPacket::decode(packet).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing audio packet: {:?}", packet);
        Ok(())
    }
}
