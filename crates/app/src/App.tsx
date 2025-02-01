import AudioDeviceSelector from "@/components/app/audio-device-selector";
import "./index.css";

function App() {
    return (
        <main className="container">
            <h1 className="text-3xl bg-primary font-bold underline">Hello world!</h1>

            <AudioDeviceSelector />
        </main>
    );
}

export default App;
