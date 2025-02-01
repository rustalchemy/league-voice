use client::{
    audio::{codec::opus::OpusAudioCodec, cpal::CpalAudioHandler},
    client::{tokio::TokioClient, Client},
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_| {
            tauri::async_runtime::spawn(setup());

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn setup() {
    let codec = match CpalAudioHandler::<OpusAudioCodec>::new() {
        Ok(codec) => codec,
        Err(e) => {
            eprintln!("Failed to create audio codec: {}", e);
            return;
        }
    };

    let addr = std::borrow::Cow::Borrowed("127.0.0.1:8080");
    let client = match TokioClient::connect(addr, codec).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            return;
        }
    };

    match client.run().await {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Client error: {}", e);
        }
    };
}
