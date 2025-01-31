use audio::{codec::opus::OpusAudioCodec, cpal::CpalAudioHandler};
use clap::Parser;
use client::{tokio::TokioClient, Client};
use error::ClientError;

mod audio;
mod client;
mod error;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Host to connect to.
    #[arg(long, default_value = "127.0.0.1:8080")]
    host: String,
}

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ClientError> {
    let args = Args::parse();
    let codec = CpalAudioHandler::<OpusAudioCodec>::new()?;
    let client = TokioClient::connect(args.host.into(), codec).await?;
    client.run().await
}
