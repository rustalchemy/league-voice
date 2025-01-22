use crate::error::ClientError;
use std::borrow::Cow;

pub mod tokio;

pub trait Client: Sized {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, ClientError>;
}
