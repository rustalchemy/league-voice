import { invoke, InvokeArgs, InvokeOptions } from "@tauri-apps/api/core";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}

export async function invoke_typed<T>(cmd: string, args?: InvokeArgs, options?: InvokeOptions): Promise<T> {
    return JSON.parse(await invoke<string>(cmd, args, options));
}
