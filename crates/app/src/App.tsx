import AudioDeviceSelector from "@/components/app/audio-device-selector";
import "./index.css";
import MenuBar from "@/components/app/menu-bar";

function App() {
    return (
        <main className="container">
            <MenuBar />
            <AudioDeviceSelector />
        </main>
    );
}

export default App;
