import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@/components/ui/select";
import { Label } from "../ui/label";

function AudioDeviceSelector() {
    const [devices, setDevices] = useState<MediaDeviceInfo[]>([]);
    const [inputDevice, setInputDevice] = useState<string | null>(null);
    const [outputDevice, setOutputDevice] = useState<string | null>(null);

    useEffect(() => {
        async function getDevices() {
            try {
                await navigator.mediaDevices.getUserMedia({ audio: true });
                const mediaDevices = await navigator.mediaDevices.enumerateDevices();
                const audioDevices = mediaDevices.filter((device) => device.kind.includes("audio"));
                setDevices(audioDevices);
            } catch (err) {
                console.error("Error fetching devices:", err);
            }
        }
        getDevices();
    }, []);

    async function handleOutputSelect(deviceId: string) {
        setOutputDevice(deviceId);
        try {
            await invoke("", { deviceId });
        } catch (err) {
            console.error("Failed to set output device:", err);
        }
    }

    return (
        <div className="p-4 flex flex-col gap-2">
            <h2 className="text-lg font-bold mb-2">Audio Device Selector</h2>
            <Label htmlFor="microphone">Microphone</Label>
            <Select onValueChange={setInputDevice}>
                <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select Microphone" />
                </SelectTrigger>
                <SelectContent>
                    {devices
                        .filter((d) => d.kind === "audioinput")
                        .map((device) => (
                            <SelectItem key={device.deviceId} value={device.deviceId}>
                                {device.label || "Unknown Microphone"}
                            </SelectItem>
                        ))}
                </SelectContent>
            </Select>

            <Label htmlFor="speakers">Speakers</Label>
            <Select onValueChange={handleOutputSelect}>
                <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select Output" />
                </SelectTrigger>
                <SelectContent>
                    {devices
                        .filter((d) => d.kind === "audiooutput")
                        .map((device) => (
                            <SelectItem key={device.deviceId} value={device.deviceId}>
                                {device.label || "Unknown Output Device"}
                            </SelectItem>
                        ))}
                </SelectContent>
            </Select>
        </div>
    );
}

export default AudioDeviceSelector;
