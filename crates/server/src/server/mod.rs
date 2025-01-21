use crate::error::ServerError;
use common::packet::Packet;
use std::borrow::Cow;

pub mod tokio;

pub trait Server: Sized {
    async fn run(addr: Cow<'_, str>) -> Result<Self, ServerError>;
    async fn process_packet(packet: Packet) -> Result<(), ServerError>;
}
