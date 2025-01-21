use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("failed to bind: {0}")]
    FailedToBind(std::io::Error),

    #[error("failed to join: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}
