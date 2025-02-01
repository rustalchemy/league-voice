import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@/components/ui/select";
import { Label } from "../ui/label";
import { Button } from "../ui/button";

enum DeviceType {
    Input = "Input",
    Output = "Output",
}

interface DeviceInfo {
    name: string;
    device_type: DeviceType;
    active: boolean;
    default: boolean;
}

async function invoke_typed<T>(cmd: string): Promise<T> {
    let res = await invoke<string>(cmd);
    let data: T = JSON.parse(res);
    return data;
}

function AudioDeviceSelector() {
    const [devices, setDevices] = useState<Array<DeviceInfo>>(new Array<DeviceInfo>());
    const [inputDevice, setInputDevice] = useState<string | undefined>(undefined);
    const [outputDevice, setOutputDevice] = useState<string | undefined>(undefined);

    useEffect(() => {
        async function getDevices() {
            try {
                let res = await invoke_typed<Array<DeviceInfo>>("get_devices");
                console.log(res);
                setDevices(res || new Array<DeviceInfo>());

                let input = res.find((d) => d.device_type === DeviceType.Input && d.active);
                setInputDevice(input?.name);

                let output = res.find((d) => d.device_type === DeviceType.Output && d.active);
                setOutputDevice(output?.name);
            } catch (err) {
                console.error("Error fetching devices:", err);
            }
        }
        getDevices();
    }, []);

    async function handleOutputSelect(deviceId: string) {
        setOutputDevice(deviceId);
        try {
            let res = await invoke("set_device", { deviceName: deviceId });
            console.log(res);
        } catch (err) {
            console.error("Failed to set output device:", err);
        }
    }

    return (
        <div className="p-4 flex flex-col gap-2">
            <h2 className="text-lg font-bold mb-2">Audio Device Selector</h2>
            <Label htmlFor="microphone">Microphone</Label>
            <Select onValueChange={setInputDevice} value={inputDevice}>
                <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select Microphone" />
                </SelectTrigger>
                <SelectContent>
                    {devices
                        .filter((d) => d.device_type === DeviceType.Input)
                        .map((device) => (
                            <SelectItem key={device.name} value={device.name}>
                                {device.name || "Unknown Microphone"}
                            </SelectItem>
                        ))}
                </SelectContent>
            </Select>

            <Label htmlFor="speakers">Speakers</Label>
            <Select onValueChange={handleOutputSelect} value={outputDevice}>
                <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select Output" />
                </SelectTrigger>
                <SelectContent>
                    {devices
                        .filter((d) => d.device_type === DeviceType.Output)
                        .map((device) => (
                            <SelectItem key={device.name} value={device.name}>
                                {device.name || "Unknown output"}
                            </SelectItem>
                        ))}
                </SelectContent>
            </Select>

            <Button
                onClick={() => {
                    handleOutputSelect(inputDevice || outputDevice || "");
                }}
            >
                Test
            </Button>
        </div>
    );
}

export default AudioDeviceSelector;
