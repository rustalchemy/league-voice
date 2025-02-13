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
    println!("Default device: {:?}", default_name);
    println!("Devices:");

    let mut devices = Vec::new();
    for device in host_devices {
        let name = device.name().unwrap_or_default();
        println!("{:?}", name);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_host() {
        let host = get_host().unwrap();
        assert_eq!(
            host.id(),
            cpal::available_hosts().into_iter().next().unwrap()
        );
    }

    #[test]
    fn test_get_host_devices() {
        let host = get_host().unwrap();
        assert!(!get_host_devices(&DeviceType::Input, &host)
            .unwrap()
            .is_empty());
        assert!(!get_host_devices(&DeviceType::Output, &host)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_get_device_config() {
        let input_devices = get_host_devices(&DeviceType::Input, &get_host().unwrap()).unwrap();
        assert_eq!(
            get_device_config(&input_devices[0].name, &input_devices)
                .unwrap()
                .name,
            input_devices[0].name
        );
    }

    #[test]
    fn test_init_device_type() {
        let devices_info = init_device_type(DeviceType::Input, &get_host().unwrap()).unwrap();
        assert!(!devices_info.is_empty());
        assert!(devices_info.iter().any(|device| device.active));
    }

    // #[tokio::test]
    // async fn test_setup_input_stream() {
    //     let devices_info = get_host_devices(&DeviceType::Input, &get_host().unwrap()).unwrap();
    //     let device_info = get_device_config(&devices_info[0].name, &devices_info).unwrap();
    //     let device = device_info.device.as_ref().unwrap();
    //     let (mic_tx, mut rx) = mpsc::channel(100);
    //     let stream = setup_input_stream(&device, &device_info, mic_tx).unwrap();
    //     stream.play().unwrap();
    //     assert!(rx.recv().await.is_some());
    //     stream.pause().unwrap();
    //     sleep(Duration::from_millis(150)).await;
    //     assert!(rx.try_recv().is_err());
    // }

    // #[tokio::test]
    // async fn test_setup_output_stream() {
    //     let host = get_host().unwrap();
    //     let devices_info = get_host_devices(&DeviceType::Output, &host).unwrap();
    //     let device_info = get_device_config(&devices_info[0].name, &devices_info).unwrap();
    //     let device = device_info.device.as_ref().unwrap();
    //     let (output_tx, output_rx) = std::sync::mpsc::channel();
    //     let stream = setup_output_stream(&device, &device_info, output_rx).unwrap();
    //     stream.play().unwrap();

    //     sleep(Duration::from_millis(150)).await;
    //     assert!(output_tx.send(vec![0.0; 100]).is_ok());
    //     stream.pause().unwrap();
    // }

    #[test]
    fn test_get_device_config_no_device() {
        let devices_info = get_host_devices(&DeviceType::Input, &get_host().unwrap()).unwrap();
        assert!(get_device_config("no_device", &devices_info).is_err());
    }
}
