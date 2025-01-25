use crate::error::ServerError;
use ::tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;

pub type Clients = Mutex<HashMap<Uuid, Client>>;

pub struct Client {
    pub(super) id: Uuid,
    pub(super) write_tx: mpsc::Sender<Vec<u8>>,
}

impl Client {
    pub fn new(id: Uuid, write_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { id, write_tx }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub async fn send(&self, packet: &[u8]) -> Result<(), ServerError> {
        match self.write_tx.send(packet.to_vec()).await {
            Ok(_) => Ok(()),
            Err(_) => Err(ServerError::ClientSendError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::tokio::sync::mpsc;

    #[tokio::test]
    async fn test_client_send() {
        let (tx, mut rx) = mpsc::channel(1);
        let client = Client::new(Uuid::new_v4(), tx);

        let packet = vec![0, 1, 2, 3];
        client.send(&packet).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(packet, received);
    }

    #[test]
    fn test_client_id() {
        let (tx, mut _rx) = mpsc::channel(1);
        let id = Uuid::new_v4();
        let client = Client::new(id, tx);

        assert_eq!(id, client.id());
    }

    #[test]
    fn test_client_new() {
        let id = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(1);
        let client = Client::new(id, tx);

        assert_eq!(id, client.id());
    }
    #[tokio::test]
    async fn test_client_send_error() {
        let (tx, _) = mpsc::channel(1);
        let client = Client::new(Uuid::new_v4(), tx);

        let packet = vec![0, 1, 2, 3];
        assert!(
            client.send(&packet).await.is_err(),
            "expected send to fail with ClientSendError"
        );
    }
}
