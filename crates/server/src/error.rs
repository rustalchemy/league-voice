use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("failed to bind: {0}")]
    FailedToBind(std::io::Error),

    #[error("failed to join: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("failed to process packet: {0}")]
    FailedToProcessPacket(String),

    #[error("connection closed by peer")]
    ConnectionClosedByPeer,

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed to decode packet: {0}")]
    FailedToDecodePacket(#[from] bincode::ErrorKind),
}
