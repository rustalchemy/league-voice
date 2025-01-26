use crate::error::ClientError;

use super::AudioCodec;
use opus::{Application, Decoder, Encoder};
use std::sync::Mutex;

pub struct OpusAudioCodec {
    encoder: Mutex<Encoder>,
    decoder: Mutex<Decoder>,
}

impl AudioCodec for OpusAudioCodec {
    fn new() -> Result<Self, ClientError> {
        let mut encoder = Encoder::new(48000, opus::Channels::Mono, Application::Audio)?;
        encoder.set_bitrate(opus::Bitrate::Bits(64000)).unwrap();
        let decoder = Decoder::new(48000, opus::Channels::Mono)?;

        Ok(OpusAudioCodec {
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
        })
    }

    fn encode(&self, data: Vec<f32>) -> Result<Vec<u8>, ClientError> {
        let mut encoded = vec![0; 960];
        let mut encoder: std::sync::MutexGuard<'_, Encoder> = self.encoder.lock().unwrap();
        let len = encoder
            .encode_float(&data, &mut encoded)
            .map_err(|e| ClientError::EncodeError(e.to_string()))?;
        encoded.truncate(len);
        Ok(encoded)
    }

    fn decode(&self, data: Vec<u8>) -> Result<Vec<f32>, ClientError> {
        let mut decoded = vec![0.0; 960];
        let mut decoder = self.decoder.lock().unwrap();
        let len = decoder
            .decode_float(&data, &mut decoded, false)
            .map_err(|e| ClientError::DecodeError(e.to_string()))?;
        decoded.truncate(len);
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_new_opus_audio_codec() {
        let _codec = OpusAudioCodec::new();
    }
}
