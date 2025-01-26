use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    // #[error("invalid packet: possible protocol error")]
    // InvalidPacket,
    #[error("buffer overflow: possible protocol error")]
    BufferOverflow,

    #[error("connection closed by peer")]
    ConnectionClosedByPeer,

    #[error("failed on io: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed on bincode: {0}")]
    BincodeError(#[from] Box<bincode::ErrorKind>),
}
