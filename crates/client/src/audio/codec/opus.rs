use super::AudioCodec;
use crate::error::ClientError;
use opus::{Application, Decoder, Encoder};
use std::sync::Mutex;

#[derive(Debug)]
pub struct OpusAudioCodec {
    encoder: Mutex<Option<Encoder>>,
    decoder: Mutex<Option<Decoder>>,

    frame_size: Option<usize>,
}

impl AudioCodec for OpusAudioCodec {
    fn new() -> Result<Self, ClientError>
    where
        Self: Sized,
    {
        Ok(Self {
            encoder: Mutex::new(None),
            decoder: Mutex::new(None),
            frame_size: None,
        })
    }

    fn update(&mut self, sample_rate: u32, channels: usize) -> Result<(), ClientError> {
        let channel = match channels {
            1 => opus::Channels::Mono,
            2 => opus::Channels::Stereo,
            _ => return Err(ClientError::InvalidChannelCount),
        };

        let sample_rate = match sample_rate {
            8000 | 12000 | 16000 | 24000 | 48000 => sample_rate,
            _ => 48000,
        };

        println!(
            "Creating Opus codec with sample rate: {}, channels: {}",
            sample_rate, channels
        );

        {
            let mut encoder = match self.encoder.lock() {
                Ok(encoder) => encoder,
                Err(err) => {
                    eprintln!("Failed to lock encoder: {}", err);
                    return Err(ClientError::PoisonedLock);
                }
            };

            *encoder = Some(
                match Encoder::new(sample_rate, channel, Application::Audio) {
                    Ok(encoder) => encoder,
                    Err(err) => {
                        eprintln!("Failed to create Opus encoder: {}", err);
                        return Err(ClientError::OpusError(err));
                    }
                },
            );
        }
        {
            let mut decoder = match self.decoder.lock() {
                Ok(decoder) => decoder,
                Err(err) => {
                    eprintln!("Failed to lock decoder: {}", err);
                    return Err(ClientError::PoisonedLock);
                }
            };

            *decoder = Some(match Decoder::new(sample_rate, channel) {
                Ok(decoder) => decoder,
                Err(err) => {
                    eprintln!("Failed to create Opus decoder: {}", err);
                    return Err(ClientError::OpusError(err));
                }
            });
        }

        self.frame_size = Some((sample_rate as usize / 1000 * 20) * channels);

        Ok(())
    }

    fn encode(&self, data: Vec<f32>) -> Result<Vec<u8>, ClientError> {
        let mut encoder = match self.encoder.lock() {
            Ok(encoder) => encoder,
            Err(_) => return Err(ClientError::PoisonedLock),
        };

        if encoder.is_none() {
            return Err(ClientError::CodecNotInitialized);
        }

        if self.frame_size.is_none() {
            return Err(ClientError::InvalidFrameSize);
        }

        let frame_size = self.frame_size.as_ref().take().unwrap();
        let mut encoded = vec![0; *frame_size];

        let encoder = encoder.as_mut().take().unwrap();
        let len = encoder.encode_float(&data, &mut encoded)?;
        encoded.truncate(len);
        Ok(encoded)
    }

    fn decode(&self, data: Vec<u8>) -> Result<Vec<f32>, ClientError> {
        let mut decoder = match self.decoder.lock() {
            Ok(decoder) => decoder,
            Err(_) => return Err(ClientError::PoisonedLock),
        };

        if decoder.is_none() {
            return Err(ClientError::CodecNotInitialized);
        }

        if self.frame_size.is_none() {
            return Err(ClientError::InvalidFrameSize);
        }

        let frame_size = self.frame_size.as_ref().take().unwrap();
        let mut decoded = vec![0.0; *frame_size];

        let decoder = decoder.as_mut().take().unwrap();
        let len = decoder.decode_float(&data, &mut decoded, false)?;
        decoded.truncate(len);
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_new_opus_audio_codec() {
        assert!(OpusAudioCodec::new().unwrap().update(48000, 1).is_ok());
        assert!(OpusAudioCodec::new().unwrap().update(48000, 2).is_ok());
        assert!(OpusAudioCodec::new().unwrap().update(48000, 3).is_err());
    }

    #[test]
    fn should_encode_and_decode_audio_data() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();

        let data = vec![0.0; 960];
        let encoded = codec.encode(data.clone()).unwrap();
        let decoded = codec.decode(encoded).unwrap();
        for (a, b) in data.iter().zip(decoded.iter()) {
            let difference = (a - b).abs();
            assert!(difference < 0.1e-10, "Difference: {}", difference);
        }
    }

    #[test]
    fn should_encode_and_decode_audio_data_stereo() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 2).unwrap();

        let data = vec![0.0; 960 * 2];
        let encoded = codec.encode(data.clone()).unwrap();
        let decoded = codec.decode(encoded).unwrap();
        for (a, b) in data.iter().zip(decoded.iter()) {
            let difference = (a - b).abs();
            assert!(difference < 0.1e-2, "Difference: {}", difference);
        }
    }

    #[test]
    fn should_fail_to_encode_audio_data() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();
        let data = vec![0.0; 1];
        assert!(codec.encode(data).is_err());
    }

    #[test]
    fn should_fail_to_decode_audio_data() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();
        let data = vec![0; 2000];
        assert!(codec.decode(data).is_err());
    }
}
