mod tokio;

pub trait Server {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error>>;
}
