use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Deserialize, Serialize, PartialEq, Hash, Eq, Debug, Clone)]
pub enum PacketId {
    ConnectPacket = 0,
    DisconnectPacket = 1,
    AudioPacket = 2,
}

impl PacketId {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PacketId::ConnectPacket),
            1 => Some(PacketId::DisconnectPacket),
            2 => Some(PacketId::AudioPacket),
            _ => None,
        }
    }
}
