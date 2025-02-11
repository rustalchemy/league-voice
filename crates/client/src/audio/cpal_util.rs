use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, Stream,
};
use tokio::sync::mpsc;

use crate::error::ClientError;

use super::{DeviceInfo, DeviceType};

pub fn get_host() -> Result<cpal::Host, ClientError> {
    let host_id = match cpal::available_hosts().into_iter().next() {
        Some(host) => host,
        None => {
            println!("No host setup device found");
            return Err(ClientError::NoHost);
        }
    };
    Ok(cpal::host_from_id(host_id)?)
}

pub fn get_host_devices(
    device_type: &DeviceType,
    host: &cpal::Host,
) -> Result<Vec<DeviceInfo>, ClientError> {
    let host_devices = match device_type {
        DeviceType::Input => host.input_devices()?,
        DeviceType::Output => host.output_devices()?,
    };

    let default_device = match device_type {
        DeviceType::Input => host.default_input_device(),
        DeviceType::Output => host.default_output_device(),
    };

    let default_device = match default_device {
        Some(device) => device,
        None => {
            println!("No default device found");
            return Err(ClientError::NoDevice);
        }
    };
    let default_name = default_device.name().unwrap_or_default();

    let mut devices = Vec::new();
    for device in host_devices {
        let name = device.name().unwrap_or_default();
        devices.push(DeviceInfo {
            name: name.clone(),
            device_type: device_type.clone(),
            active: false,
            default: name == default_name,
            config: match device_type {
                DeviceType::Input => device.default_input_config()?,
                DeviceType::Output => device.default_output_config()?,
            },
            device: Some(device),
        });
    }
    Ok(devices)
}

pub fn get_device_config(
    device_name: &str,
    device_infos: &Vec<DeviceInfo>,
) -> Result<DeviceInfo, ClientError> {
    match device_infos
        .iter()
        .find(|device| device.name == device_name)
    {
        Some(device) => Ok(device.clone()),
        None => Err(ClientError::NoDevice),
    }
}

pub fn setup_input_stream(
    device: &Device,
    device_info: &DeviceInfo,
    mic_tx: mpsc::Sender<Vec<f32>>,
) -> Result<Stream, ClientError> {
    println!("Starting INPUT stream");
    println!("Sample rate: {:?}", device_info.config.sample_rate().0);
    println!("Buffer size: {:?}", device_info.config.buffer_size());
    println!("Sample format: {:?}", device_info.config.sample_format());
    println!("Channels: {:?}", device_info.config.channels());
    println!();
    let stream = device.build_input_stream(
        &device_info.config.config(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let _ = mic_tx.try_send(data.to_vec());
        },
        |err| eprintln!("Input stream error: {:?}", err),
        None,
    )?;
    stream.play()?;
    Ok(stream)
}

pub fn setup_output_stream(
    device: &Device,
    device_info: &DeviceInfo,
    output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
) -> Result<Stream, ClientError> {
    println!("Starting OUTPUT stream");
    println!("Sample rate: {:?}", device_info.config.sample_rate().0);
    println!("Buffer size: {:?}", device_info.config.buffer_size());
    println!("Sample format: {:?}", device_info.config.sample_format());
    println!("Channels: {:?}", device_info.config.channels());
    println!();
    let channels = device_info.config.channels();
    let stream = device.build_output_stream(
        &device_info.config.config(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if let Ok(decoded_data) = output_rx.recv() {
                for (i, frame) in data.chunks_mut(channels.into()).enumerate() {
                    if i >= decoded_data.len() {
                        break;
                    }

                    let value = decoded_data[i].to_sample();
                    for sample in frame.iter_mut() {
                        *sample = value;
                    }
                }
            }
        },
        |err| eprintln!("Output stream error: {:?}", err),
        None,
    )?;
    stream.play()?;
    Ok(stream)
}

pub fn init_device_type(
    device_type: DeviceType,
    host: &cpal::Host,
) -> Result<Vec<DeviceInfo>, ClientError> {
    let mut devices_info = get_host_devices(&device_type, &host)?;
    let default_device = devices_info.iter().find(|device| device.default).unwrap();

    let device_info = get_device_config(&default_device.name, &devices_info)?;

    let device_name = device_info.name.clone();
    let new_device_info = DeviceInfo {
        name: device_info.name,
        device_type,
        active: true,
        default: device_info.default,
        config: device_info.config,
        device: device_info.device,
    };

    for device_info in devices_info.iter_mut() {
        if device_info.name == device_name {
            *device_info = new_device_info.clone();
        } else {
            device_info.active = false;
        }
    }

    Ok(devices_info)
}
