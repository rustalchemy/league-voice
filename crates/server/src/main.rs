use server::Server;

mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = server::tokio::TokioServer::new("0.0.0.0:8080".into(), 1);
    server.run()
}
