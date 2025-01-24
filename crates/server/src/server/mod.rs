use crate::error::ServerError;
use ::tokio::sync::{mpsc, Mutex};
use common::packet::Packet;
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use uuid::Uuid;

pub mod tokio;

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

pub trait Server: Send + Sync {
    type Handlers;

    async fn run(&mut self, addr: Cow<'_, str>) -> Result<(), ServerError>;
    async fn process_packet(
        client_id: Uuid,
        handlers: Arc<Self::Handlers>,
        packet: Packet,
    ) -> Result<(), ServerError>;
    fn clients(&self) -> Arc<Clients>;
}
