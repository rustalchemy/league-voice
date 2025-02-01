use tokio::sync::Mutex;

use client::{
    audio::{codec::opus::OpusAudioCodec, cpal::CpalAudioHandler, AudioHandler},
    client::{tokio::TokioClient, Client},
};
use tauri::{Manager, State};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_devices(state: State<'_, Mutex<AppState>>) -> Result<String, ()> {
    let state = state.inner().lock().await;

    let out_devices = state
        .client
        .audio_handler()
        .get_devices(client::audio::DeviceType::Output);

    let in_devices = state
        .client
        .audio_handler()
        .get_devices(client::audio::DeviceType::Input);

    let mut devices = vec![];
    devices.extend(out_devices);
    devices.extend(in_devices);

    let devices = serde_json::to_string(&devices).unwrap();

    Ok(devices)
}

struct AppState {
    client: TokioClient<CpalAudioHandler<OpusAudioCodec>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let res = tauri::async_runtime::spawn(async move { setup().await });
            let res = tauri::async_runtime::block_on(res);

            let client = match res {
                Ok(client) => client,
                Err(e) => {
                    panic!("Failed to setup client: {}", e);
                }
            };

            let client = match client {
                Ok(client) => client,
                Err(e) => {
                    panic!("Failed to setup client: {}", e);
                }
            };

            app.manage(Mutex::new(AppState { client }));

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_devices])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn setup() -> Result<TokioClient<CpalAudioHandler<OpusAudioCodec>>, String> {
    let codec = match CpalAudioHandler::<OpusAudioCodec>::new() {
        Ok(codec) => codec,
        Err(e) => {
            eprintln!("Failed to create audio codec: {}", e);
            return Err(e.to_string());
        }
    };

    let addr = std::borrow::Cow::Borrowed("127.0.0.1:8080");
    let client = match TokioClient::connect(addr, codec).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            return Err(e.to_string());
        }
    };

    Ok(client)
}
