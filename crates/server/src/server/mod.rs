use std::borrow::Cow;

pub mod tokio;

pub trait Server: Sized {
    async fn run(addr: Cow<'_, str>) -> Result<Self, Box<dyn std::error::Error>>;
}
