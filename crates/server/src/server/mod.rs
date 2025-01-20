pub mod tokio;

pub trait Server {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn is_running(&self) -> bool;
}
