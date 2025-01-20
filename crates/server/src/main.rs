use server::Server;

mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = server::tokio::TokioServer::new("127.0.0.1:8080".into(), 0);
    server.run()
}
