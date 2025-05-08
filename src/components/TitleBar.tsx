import { getCurrentWindow } from '@tauri-apps/api/window';
import './TitleBar.css';

const TitleBar = () => {
    const appWindow = getCurrentWindow();

    return (
    <div className="titlebar" data-tauri-drag-region>
        <div className="window-controls">
        <button className="close" onClick={() => appWindow.close()}></button>
        <button className="minimize" onClick={() => appWindow.minimize()}></button>
        <button className="maximize" onClick={() => appWindow.toggleMaximize()}></button>
        </div>
    </div>
    );
};

export default TitleBar;