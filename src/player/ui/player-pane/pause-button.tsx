import { invoke } from "@tauri-apps/api";
import PlaybackButton from "../../shared-ui/playback-button";

function PauseButton() {
  const handleClick = async () => {
    await invoke("player_pause");
  };
  return <PlaybackButton title="Pause" icon="Pause" onClick={handleClick} />;
}

export default PauseButton;
