import { invoke } from "@tauri-apps/api";
import PlaybackButton from "../../../shared-ui/playback-button";

export default function PlayButton() {
  const handleClick = async () => {
    await invoke("player_play");
  };
  return <PlaybackButton title="Play" icon="Play" onClick={handleClick} />;
}
