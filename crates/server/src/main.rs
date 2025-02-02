use common::packet::ids::PacketId;
use error::ServerError;
use packets::handlers;
use server::{tokio::TokioServer, Server};
use std::borrow::Cow;

mod error;
mod packets;
pub mod server;

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let mut server = TokioServer::new();
    server.add_handler(
        PacketId::ConnectPacket,
        Box::new(handlers::connect::ConnectHandler {}),
    );
    server.add_handler(
        PacketId::AudioPacket,
        Box::new(handlers::audio::AudioHandler(server.clients().clone())),
    );
    server.add_handler(
        PacketId::DisconnectPacket,
        Box::new(handlers::disconnect::DisconnectHandler {}),
    );
    server.run(Cow::Borrowed(&addr)).await
}
