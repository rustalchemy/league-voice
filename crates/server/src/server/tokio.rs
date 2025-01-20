use std::{borrow::Cow, sync::Arc};
use tokio::net::TcpListener;

use super::Server;

pub(crate) struct TokioServer {
    runtime: tokio::runtime::Runtime,
    listener: Arc<TcpListener>,
}

impl TokioServer {
    pub fn new(addr: Cow<'_, str>, workers: usize) -> Self {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => panic!("Failed to create runtime: {}", e),
        };

        let listener = match runtime
            .block_on(async { TcpListener::bind(Cow::into_owned(addr.clone())).await })
        {
            Ok(listener) => Arc::new(listener),
            Err(e) => panic!("Failed to bind address: {}", e),
        };

        TokioServer { runtime, listener }
    }
}

impl Server for TokioServer {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = self.listener.clone();
        self.runtime.spawn(async move {
            let listener = listener.clone();
            loop {
                let (socket, _) = match listener.accept().await {
                    Ok((socket, _)) => (socket, None::<()>),
                    Err(e) => {
                        println!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
                println!("Accepted connection from: {}", socket.peer_addr().unwrap());
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpStream};

    #[test]
    fn should_open_a_server_connection_on_given_address() {
        let addr = "127.0.0.1:2";
        let mut server = TokioServer::new(Cow::Borrowed(addr), 1);
        server.run().unwrap();

        let mut client = TcpStream::connect(addr).unwrap();
        assert!(
            client.write_all(b"hello").is_ok(),
            "failed to write to server"
        );
    }

    #[test]
    fn should_fail_when_creating_tokio_runtime() {
        assert!(
            std::panic::catch_unwind(|| TokioServer::new(Cow::Borrowed("127.0.0.1:4"), 0)).is_err(),
            "expected server to panic"
        );
    }

    #[test]
    fn should_return_error_on_invalid_address() {
        let addr = "127.0.0.1:1";
        let mut server = TokioServer::new(Cow::Borrowed(addr), 1);
        server.run().unwrap();

        let server_err = std::panic::catch_unwind(|| TokioServer::new(Cow::Borrowed(addr), 1));
        assert!(server_err.is_err(), "expected an error");
    }

    #[tokio::test]
    async fn should_fail_if_server_is_initialized_on_existing_tokio_runtime() {
        assert!(
            std::panic::catch_unwind(|| TokioServer::new(Cow::Borrowed("127.0.0.1:3"), 1)).is_err(),
            "expected server to panic"
        );
    }
}
