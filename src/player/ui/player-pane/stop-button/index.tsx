import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import usePlaybackState from "../../../hooks/use-playback-state";

export default function StopButton() {
  const playbackState = usePlaybackState();
  const handleClick = async () => {
    await invoke("player_stop");
  };
  return (
    <PlaybackButton
      disabled={playbackState === "Stopped"}
      title="Stop"
      icon="stop"
      onClick={handleClick}
    />
  );
}
