import { invoke_typed } from "@/lib/utils";
import { Button } from "../ui/button";

enum WindowState {
    Minimize = "Minimize",
    Close = "Close",
}

function MenuBar() {
    async function manageWindow(state: WindowState) {
        try {
            let res = await invoke_typed<WindowState>("manage_window", { state });
            console.log(res);
        } catch (err) {
            console.error("Failed to manage window:", err);
        }
    }

    return (
        <div className="flex items-center justify-between p-2 border-b-1 border-b-gray-200">
            <p className="text-xs font-bold">Voice</p>

            <div className="flex items-center space-x-2 ml-auto">
                <Button onClick={() => manageWindow(WindowState.Minimize)} className="w-3 h-3 p-0 rounded-full bg-yellow-500"></Button>
                <Button onClick={() => manageWindow(WindowState.Minimize)} className="w-3 h-3 p-0 rounded-full bg-red-500"></Button>
            </div>
        </div>
    );
}

export default MenuBar;
