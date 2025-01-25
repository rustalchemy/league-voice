use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Deserialize, Serialize, PartialEq, Hash, Eq, Debug, Clone)]
pub enum PacketId {
    ConnectPacket = 0,
    DisconnectPacket = 1,
    AudioPacket = 2,
}

impl PacketId {
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PacketId::ConnectPacket),
            1 => Some(PacketId::DisconnectPacket),
            2 => Some(PacketId::AudioPacket),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_id_to_u8() {
        assert_eq!(PacketId::ConnectPacket.to_u8(), 0);
        assert_eq!(PacketId::DisconnectPacket.to_u8(), 1);
        assert_eq!(PacketId::AudioPacket.to_u8(), 2);
    }

    #[test]
    fn test_packet_id_from_u8() {
        assert_eq!(PacketId::from_u8(0), Some(PacketId::ConnectPacket));
        assert_eq!(PacketId::from_u8(1), Some(PacketId::DisconnectPacket));
        assert_eq!(PacketId::from_u8(2), Some(PacketId::AudioPacket));
        assert_eq!(PacketId::from_u8(3), None);
    }
}
