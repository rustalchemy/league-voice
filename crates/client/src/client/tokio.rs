use crate::{audio::AudioHandler, client::Client, error::ClientError};
use common::packet::{Packet, MAX_PACKET_SIZE};
use std::{borrow::Cow, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    select,
};

pub(crate) struct TokioClient<A: AudioHandler> {
    stream: TcpStream,
    audio_handler: Arc<A>,
}

impl<A: AudioHandler + 'static> Client<A> for TokioClient<A> {
    async fn connect(addr: Cow<'_, str>, audio_handler: A) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(Cow::into_owned(addr.clone())).await?;
        Ok(Self {
            stream,
            audio_handler: Arc::new(audio_handler),
        })
    }

    async fn run(self) -> Result<(), ClientError> {
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
        let (output_tx, output_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);

        let (mut read, mut write) = self.stream.into_split();

        let read_handle = tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
            loop {
                let mut temp_buffer = [0; MAX_PACKET_SIZE];
                let bytes_read = read.read(&mut temp_buffer).await?;
                if bytes_read == 0 {
                    return Err(ClientError::ConnectionClosedByPeer);
                }

                // println!("Bytes read: {}", bytes_read);
                buffer.extend_from_slice(&temp_buffer[..bytes_read]);

                while let Ok(packet) = Packet::decode(&mut buffer) {
                    println!("Packet: {:?}", packet.data);

                    match output_tx.send(packet.data).await {
                        Ok(_) => {}
                        Err(err) => eprintln!("Failed to send audio packet: {:?}", err),
                    };
                }

                if buffer.len() > MAX_PACKET_SIZE * 2 {
                    println!("Buffer length: {}", buffer.len());
                    return Err(ClientError::BufferOverflow);
                }
            }
        });

        let write_handle = tokio::spawn(async move {
            while let Some(packet) = write_rx.recv().await {
                write.write_all(&packet).await?;
                write.flush().await?;
            }
            Ok(())
        });

        let audio_handler = self.audio_handler.clone();
        let microphone_handle =
            tokio::spawn(async move { audio_handler.retrieve(write_tx, output_rx).await });

        select! {
            Ok(read_result) = read_handle => {
                println!("Read result: {:?}", read_result);
                read_result
            },
            Ok(write_result) = write_handle => {
                println!("Write result: {:?}", write_result);
                write_result
            }
            Ok(microphone_result) = microphone_handle => {
                println!("Microphone result: {:?}", microphone_result);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{codec::opus::OpusAudioCodec, cpal::CpalAudioHandler};
    use tokio::select;

    #[tokio::test]
    async fn test_tokio_client_connect() {
        let addr = "127.0.0.1:8111";

        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);

            Ok::<(), std::io::Error>(())
        });
        let client = tokio::spawn(async move {
            let client =
                TokioClient::connect(addr.into(), CpalAudioHandler::<OpusAudioCodec>::new()).await;
            client
        });
        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
        }
    }
}
