use crate::packet::{ids::PacketId, packet_type::PacketType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct ConnectPacket;

impl PacketType for ConnectPacket {
    fn packet_id() -> PacketId {
        PacketId::ConnectPacket
    }
}
