import { useState } from "react";

function MenuBar() {
    function handleClose() {
        console.log("Window closed");
    }

    function handleMinimize() {
        console.log("Window minimized");
    }

    return (
        <div className="flex items-center justify-between p-2">
            <div className="flex items-center space-x-2 ml-auto">
                {/* Minimize Button */}
                <button onClick={handleMinimize} className="w-3 h-3 rounded-full bg-yellow-500"></button>

                <button onClick={handleClose} className="w-3 h-3 rounded-full bg-red-500"></button>
            </div>
        </div>
    );
}

export default MenuBar;
