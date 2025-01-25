use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("failed to join: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("failed to process packet: Buffer overflow: possible protocol error")]
    FailedToProcessPacket,

    #[error("connection closed by peer")]
    ConnectionClosedByPeer,

    #[error("handler not found for packet id")]
    HandlerNotFound,

    #[error("invalid packet")]
    InvalidPacket,

    #[error("invalid packet id for handler")]
    InvalidHandlerPacketId,

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed to decode packet type: {0}")]
    FailedToDecodePacketType(#[from] Box<bincode::ErrorKind>),

    #[error("failed to send to client")]
    ClientSendError,
}
