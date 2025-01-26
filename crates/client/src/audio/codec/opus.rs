use super::AudioCodec;
use opus::{Application, Decoder, Encoder};
use std::sync::Mutex;

pub struct OpusAudioCodec {
    encoder: Mutex<Encoder>,
    decoder: Mutex<Decoder>,
}

impl AudioCodec for OpusAudioCodec {
    fn new() -> Self {
        let mut encoder = Encoder::new(48000, opus::Channels::Mono, Application::Audio)
            .expect("Failed to create Opus decoder");
        encoder.set_bitrate(opus::Bitrate::Bits(64000)).unwrap();

        let decoder =
            Decoder::new(48000, opus::Channels::Mono).expect("Failed to create Opus decoder");

        OpusAudioCodec {
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
        }
    }

    fn encode(&self, data: Vec<f32>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut encoded = vec![0; 4000];
        let mut encoder: std::sync::MutexGuard<'_, Encoder> = self.encoder.lock().unwrap();
        let len = encoder.encode_float(&data, &mut encoded)?;
        encoded.truncate(len);
        println!("Encoded length: {} {:?}", encoded.len(), encoded);
        Ok(encoded)
    }

    fn decode(&self, data: Vec<u8>) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut decoded = vec![0.0; 960];
        println!("Decoding data: {:?} {:?}", data.len(), data);
        let mut decoder = self.decoder.lock().unwrap();
        let len = decoder.decode_float(&data, &mut decoded, false)?;
        decoded.truncate(len);
        println!("Decoded length: {}", decoded.len());
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_new_opus_audio_codec() {
        let codec = OpusAudioCodec::new();
    }

    #[test]
    fn should_encode_and_decode_opus_audio_data() {
        let data = [0.5; 2 * 480];

        let mut encoder = Encoder::new(48000, opus::Channels::Mono, Application::Audio)
            .expect("Failed to create Opus encoder");

        println!("Encoder: {:?}", encoder);
        let bitrate = encoder.get_bitrate().unwrap();
        println!("Bitrate: {:?}", bitrate);

        let mut decoder =
            Decoder::new(48000, opus::Channels::Mono).expect("Failed to create Opus decoder");

        let mut encoded = vec![0; 2 * 480]; // Allocate enough space for encoded data
        let encoded_len = match encoder.encode_float(&data, &mut encoded) {
            Ok(len) => len,
            Err(err) => {
                eprintln!("Failed to encode Opus data: {:?}", err);
                return;
            }
        };
        encoded.truncate(encoded_len); // Truncate the encoded vector to the actual encoded length
        println!("Encoded length: {}", encoded.len());

        let mut decoded: Vec<f32> = vec![0.0; 2 * 480];
        let decoded_len = match decoder.decode_float(&encoded, &mut decoded, false) {
            Ok(len) => len,
            Err(err) => {
                eprintln!("Failed to decode Opus data: {:?}", err);
                return;
            }
        };

        println!("Decoded length: {}", encoded.len());
        println!("Encoded data: {:?}", encoded);
        println!();
        println!("Decoded data: {:?}", decoded);

        // Compare the original and decoded data with a tolerance
        let tolerance = 1e-4; // Adjusted tolerance value
        for (i, &sample) in data.iter().enumerate() {
            assert!(
                (sample - decoded[i]).abs() < tolerance,
                "Sample at index {} differs: original = {}, decoded = {}",
                i,
                sample,
                decoded[i]
            );
        }
    }
}
