use client::{
    audio::{
        codec::opus::OpusAudioCodec, cpal::CpalAudioHandler, cpal_device::CpalDeviceHandler,
        DeviceHandler, DeviceType,
    },
    client::{tokio::TokioClient, Client},
};
use tauri::{Manager, Runtime, State, Window};
use tokio::sync::Mutex;

#[derive(Debug, serde::Deserialize)]
enum WindowState {
    Minimize,
    Close,
}

#[tauri::command]
async fn manage_window<R: Runtime>(state: WindowState, window: Window<R>) {
    match state {
        WindowState::Minimize => {
            window.minimize().unwrap();
        }
        WindowState::Close => {
            window.destroy().unwrap();
        }
    }
}

#[tauri::command]
async fn start(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.inner().lock().await;

    match state.client.run().await {
        Ok(_) => {}
        Err(e) => return Err(e.to_string()),
    }

    Ok(())
}

#[tauri::command]
async fn stop(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.inner().lock().await;

    match state.client.stop().await {
        Ok(_) => {}
        Err(e) => return Err(e.to_string()),
    }

    Ok(())
}

#[tauri::command]
async fn is_running(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.inner().lock().await;
    Ok(state.client.is_running().await)
}

#[tauri::command]
async fn get_devices(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let state = state.inner().lock().await;

    let audio_handler = state.client.device_handler();

    let out_devices = audio_handler.get_devices(client::audio::DeviceType::Output);
    let in_devices = audio_handler.get_devices(client::audio::DeviceType::Input);

    let mut devices = vec![];
    devices.extend(out_devices);
    devices.extend(in_devices);

    let devices = serde_json::to_string(&devices).unwrap();

    Ok(devices)
}

#[tauri::command]
async fn set_device(
    device_type: DeviceType,
    device_name: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.inner().lock().await;
    state.client.stop().await.unwrap();

    let audio_handler = state.client.device_handler_mut();
    match audio_handler
        .set_active_device(&device_type, device_name)
        .await
    {
        Ok(_) => {}
        Err(e) => return Err(e.to_string()),
    };

    Ok(())
}

struct AppState {
    client: TokioClient<CpalAudioHandler<OpusAudioCodec>, CpalDeviceHandler>,
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
        .invoke_handler(tauri::generate_handler![
            get_devices,
            set_device,
            is_running,
            start,
            stop,
            manage_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn setup() -> Result<TokioClient<CpalAudioHandler<OpusAudioCodec>, CpalDeviceHandler>, String>
{
    let addr = std::borrow::Cow::Borrowed("127.0.0.1:8080");
    let client = match TokioClient::connect(addr).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            return Err(e.to_string());
        }
    };

    Ok(client)
}
