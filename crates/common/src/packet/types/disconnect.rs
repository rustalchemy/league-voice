use crate::packet::{ids::PacketId, packet_type::PacketType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct DisconnectPacket;

impl PacketType for DisconnectPacket {
    fn packet_id() -> PacketId {
        PacketId::DisconnectPacket
    }
}
