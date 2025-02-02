use crate::{
    audio::{AudioHandler, DeviceHandler},
    client::Client,
    error::ClientError,
};
use common::packet::{Packet, MAX_PACKET_SIZE};
use std::{borrow::Cow, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    select,
};

pub struct TokioClient<A: AudioHandler, D: DeviceHandler> {
    stream: Option<TcpStream>,
    audio_handler: Arc<A>,
    device_handler: D,

    stop_tx: Option<tokio::sync::mpsc::Sender<()>>,
}

#[async_trait::async_trait]
impl<A: AudioHandler + 'static, D: DeviceHandler + 'static> Client<A, D> for TokioClient<A, D> {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(Cow::into_owned(addr.clone())).await?;

        Ok(Self {
            stream: Some(stream),
            audio_handler: Arc::new(A::new()?),
            device_handler: D::new()?,
            stop_tx: None,
        })
    }

    async fn run(&mut self) -> Result<(), ClientError> {
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let (mic_tx, mic_rx) = tokio::sync::mpsc::channel::<Vec<f32>>(20);
        let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        self.device_handler
            .start_defaults(mic_tx, output_rx)
            .await?;

        let (packet_sender, mut message_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
        let (mesage_transmitter, packet_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(32);

        let stream = self.stream.take().unwrap();
        let (mut read, mut write) = stream.into_split();

        let read_handle = tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
            loop {
                let mut temp_buffer = [0; MAX_PACKET_SIZE];
                let bytes_read = read.read(&mut temp_buffer).await?;
                if bytes_read == 0 {
                    return Err(ClientError::ConnectionClosedByPeer);
                }

                buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                while let Ok(packet) = Packet::decode(&mut buffer) {
                    mesage_transmitter.send(packet.data).await?;
                }

                if buffer.len() > MAX_PACKET_SIZE * 2 {
                    println!("Buffer length: {}", buffer.len());
                    return Err(ClientError::BufferOverflow);
                }
            }
        });

        let write_handle = tokio::spawn(async move {
            while let Some(packet) = message_receiver.recv().await {
                write.write_all(&packet).await?;
                write.flush().await?;
            }
            Ok(())
        });

        let audio_handler = self.audio_handler.clone();
        let microphone_handle = tokio::spawn(async move {
            audio_handler
                .start(packet_sender, packet_receiver, mic_rx, output_tx)
                .await
        });

        let stop_handler = tokio::spawn(async move {
            stop_rx.recv().await;
            Ok(())
        });

        tokio::spawn(async move {
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
                Ok(stop_result) = stop_handler => {
                    println!("Stop result: {:?}", stop_result);
                    stop_result
                }
            }
        });

        Ok(())
    }

    fn device_handler(&self) -> &D {
        &self.device_handler
    }

    fn device_handler_mut(&mut self) -> &mut D {
        &mut self.device_handler
    }

    async fn stop(&mut self) -> Result<(), ClientError> {
        let stop_tx = match &self.stop_tx {
            Some(tx) => tx,
            None => return Ok(()),
        };

        match stop_tx.send(()).await {
            Ok(_) => {}
            Err(e) => {
                panic!("Failed to send stop signal: {}", e);
            }
        }

        self.stop_tx = None;

        self.audio_handler.stop().await?;
        Ok(())
    }

    async fn is_running(&self) -> bool {
        self.stop_tx.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{
        codec::opus::OpusAudioCodec, cpal::CpalAudioHandler, cpal_device::CpalDeviceHandler,
    };
    use common::packet::AudioPacket;
    use std::time::Duration;
    use tokio::{select, time::sleep};

    pub type TokoClient = TokioClient<CpalAudioHandler<OpusAudioCodec>, CpalDeviceHandler>;

    #[tokio::test]
    async fn test_tokio_client_connect() {
        let addr = "127.0.0.1:8111";

        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();

            let packet = Packet::new(AudioPacket {
                track: vec![0; 960],
            })
            .unwrap()
            .encode();

            for _ in 0..10 {
                socket.write_all(&packet).await.unwrap();
            }
            socket.flush().await.unwrap();

            Ok::<(), std::io::Error>(())
        });
        let client = tokio::spawn(async move {
            let mut client = TokoClient::connect(addr.into()).await.unwrap();
            client.run().await
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

    #[tokio::test]
    async fn test_tokio_client_connect_fail_buffer_zero() {
        let addr = "127.0.0.1:8112";

        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();

            drop(socket);

            sleep(Duration::from_millis(10)).await;
            Ok::<(), std::io::Error>(())
        });
        let client = tokio::spawn(async move {
            let mut client = TokoClient::connect(addr.into()).await.unwrap();
            client.run().await
        });
        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
            },
            Ok(result) = client => {
                assert!(result.is_err(), "expected client to error");
            }
        }
    }

    #[tokio::test]
    async fn test_tokio_client_connect_fail_buffer_overflow() {
        let addr = "127.0.0.1:8113";

        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            let packet = [1; 4 * 1024];
            for _ in 0..10 {
                socket.write_all(&packet).await.unwrap();
            }
            socket.flush().await.unwrap();

            sleep(Duration::from_millis(10)).await;

            Ok::<(), std::io::Error>(())
        });
        let client = tokio::spawn(async move {
            let mut client = TokoClient::connect(addr.into()).await.unwrap();
            client.run().await
        });
        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
            },
            Ok(result) = client => {
                assert!(result.is_err(), "expected client to error");
            }
        }
    }
}
