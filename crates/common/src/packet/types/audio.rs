use crate::packet::{ids::PacketId, packet_type::PacketType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq, Default)]
pub struct AudioPacket {
    pub track: Vec<u8>,
}

impl PacketType for AudioPacket {
    fn packet_id() -> PacketId {
        PacketId::AudioPacket
    }
}
