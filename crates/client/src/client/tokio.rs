use crate::{
    audio::{codec::AudioCodec, DeviceHandler, DeviceType, SoundProcessor},
    client::Client,
    error::ClientError,
    handlers::audio::AudioPacketHandler,
};
use common::packet::{ids::PacketId, Packet, MAX_PACKET_SIZE};
use std::{borrow::Cow, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    select,
    sync::{broadcast, mpsc, oneshot},
};

pub struct TokioClient<A: SoundProcessor, D: DeviceHandler> {
    audio_handler: Arc<A>,
    device_handler: D,

    stop_tx: Option<oneshot::Sender<()>>,

    packet_sender: mpsc::Sender<Packet>,
    chan_output_rx: Arc<broadcast::Receiver<Vec<f32>>>,
}

#[async_trait::async_trait]
impl<A: SoundProcessor + 'static, D: DeviceHandler + 'static> Client<A, D> for TokioClient<A, D> {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(Cow::into_owned(addr.clone())).await?;
        println!("Connected to server: {}", addr);

        let (packet_sender, mut message_receiver) = mpsc::channel::<Packet>(32);
        let (chan_output_tx, chan_output_rx) = broadcast::channel::<Vec<f32>>(32);

        let audio_handler: Arc<A> = Arc::new(A::new()?);
        let audio_handler_clone = audio_handler.clone();
        let (mut read, mut write) = stream.into_split();

        let read_handle = tokio::spawn(async move {
            println!("Started reading from server");
            let audio_handler = audio_handler_clone.clone();
            let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
            loop {
                let mut temp_buffer = [0; MAX_PACKET_SIZE];
                let bytes_read = read.read(&mut temp_buffer).await?;
                if bytes_read == 0 {
                    return Err(ClientError::ConnectionClosedByPeer);
                }

                buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                loop {
                    let packet = match Packet::decode(&mut buffer) {
                        Ok(packet) => packet,
                        Err(err) => {
                            println!("Failed to decode packet {:?}", err);
                            break;
                        }
                    };
                    let packet_type = match PacketId::from_u8(packet.packet_id) {
                        Some(packet_type) => packet_type,
                        None => return Err(ClientError::InvalidPacket),
                    };

                    match packet_type {
                        PacketId::AudioPacket => {
                            AudioPacketHandler::handle_packet(
                                packet,
                                audio_handler.get_codec(),
                                chan_output_tx.clone(),
                            )
                            .await?;
                        }
                        _ => {
                            println!("Unknown packet type: {:?}", packet_type);
                        }
                    }
                }

                if buffer.len() > MAX_PACKET_SIZE * 2 {
                    println!("Buffer length: {}", buffer.len());
                    return Err(ClientError::BufferOverflow);
                }
            }
        });

        let write_handle = tokio::spawn(async move {
            println!("Started writing to server");
            while let Some(packet) = message_receiver.recv().await {
                write.write_all(&packet.encode()).await?;
                write.flush().await?;
            }
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
            }
        });

        Ok(Self {
            audio_handler,
            device_handler: D::new()?,
            stop_tx: None,
            packet_sender,
            chan_output_rx: Arc::new(chan_output_rx),
        })
    }

    async fn run(&mut self) -> Result<(), ClientError> {
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        self.stop_tx = Some(stop_tx);

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        self.device_handler.start_actives(mic_tx, output_rx)?;

        {
            let input_device = match self.device_handler.get_active_device(DeviceType::Input) {
                Some(device) => device,
                None => return Err(ClientError::NoDevice),
            };

            self.audio_handler.get_codec().lock().await.update(
                input_device.config().sample_rate().0,
                input_device.config().channels() as usize,
            )?;
        }

        let audio_handler = self.audio_handler.clone();
        let packet_sender = self.packet_sender.clone();
        let microphone_handle = tokio::spawn(async move {
            let _ = audio_handler.start(mic_rx, packet_sender).await;
            println!("audio stop")
        });

        let chan_output_rx = self.chan_output_rx.clone();
        let output_handle = tokio::spawn(async move {
            let mut output_rx = chan_output_rx.resubscribe();
            while let Ok(track) = output_rx.recv().await {
                if let Err(_) = output_tx.send(track) {
                    break;
                }
            }
            Ok::<(), ClientError>(())
        });

        select! {
            Ok(microphone_result) = microphone_handle => {
                println!("Microphone result: {:?}", microphone_result);
                Ok(())
            }
            Ok(stop_result) = stop_rx => {
                println!("Stop result: {:?}", stop_result);
                Ok(())
            }
            Ok(output_result) = output_handle => {
                println!("Output result: {:?}", output_result);
                Ok(())
            }
        }
    }

    fn device_handler(&self) -> &D {
        &self.device_handler
    }

    fn device_handler_mut(&mut self) -> &mut D {
        &mut self.device_handler
    }

    async fn stop(&mut self) -> Result<(), ClientError> {
        let stop_tx = match self.stop_tx.take() {
            Some(tx) => tx,
            None => return Ok(()),
        };

        match stop_tx.send(()) {
            Ok(_) => {}
            Err(_) => {
                panic!("Failed to send stop signal");
            }
        }

        self.stop_tx = None;
        self.audio_handler.stop().await;
        self.device_handler.stop()?;

        Ok(())
    }

    async fn is_running(&self) -> bool {
        self.stop_tx.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;

    use common::packet::{AudioPacket, Packet};
    use tokio::{io::AsyncWriteExt, select};

    use crate::{
        audio::{
            codec::opus::OpusAudioCodec, cpal_device::CpalDeviceHandler, processor::AudioProcessor,
        },
        client::Client,
        error::ClientError,
    };

    use super::TokioClient;

    pub type TokoClient = TokioClient<AudioProcessor<OpusAudioCodec>, CpalDeviceHandler>;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_tokio_client_connect() -> Result<(), ClientError> {
        let addr = "127.0.0.1:8111";

        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();

            let packet = Packet::new(AudioPacket {
                track: vec![0; 960],
            })
            .encode();

            for _ in 0..10 {
                socket.write_all(&packet).await.unwrap();
            }
            socket.flush().await.unwrap();

            Ok::<(), ClientError>(())
        });
        let client = tokio::spawn(async move {
            let mut client = TokoClient::connect(addr.into()).await.unwrap();
            client.run().await
        });
        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
                result
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to fail");
                result
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_tokio_client_connect_fail_buffer_overflow() {
        let addr = "127.0.0.1:8112";

        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();

            let packet = [1; 4 * 1024];

            for _ in 0..10 {
                socket.write_all(&packet).await.unwrap();
            }
            socket.flush().await.unwrap();

            sleep(std::time::Duration::from_millis(100));
            Ok::<(), ClientError>(())
        });

        let mut client = TokoClient::connect(addr.into()).await.unwrap();
        assert!(client.run().await.is_ok(), "expected client to fail");
    }
}
