use std::sync::Arc;

use common::packet::{packet_type::PacketType, AudioPacket, Packet};
use tokio::sync::Mutex;

use crate::{audio::codec::AudioCodec, error::ClientError};

pub struct AudioPacketHandler {}

impl AudioPacketHandler {
    pub async fn handle_packet<A: AudioCodec>(
        packet: Packet,
        codec: Arc<Mutex<A>>,
        audio_output_tx: tokio::sync::mpsc::Sender<Vec<f32>>,
    ) -> Result<(), ClientError> {
        let audio_packet = AudioPacket::decode(&packet.data)?;

        let codec = codec.lock().await;
        if let Ok(decoded_data) = codec.decode(audio_packet.track) {
            match audio_output_tx.send(decoded_data).await {
                Ok(_) => {}
                Err(_) => {
                    return Err(ClientError::InvalidPacket);
                }
            }
        }
        Ok(())
    }
}
