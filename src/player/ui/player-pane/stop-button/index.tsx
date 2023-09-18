import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import usePlaybackState from "../../../hooks/use-playback-state";

export default function StopButton() {
  const playbackState = usePlaybackState();
  const disabled = playbackState === "Stopped";
  const handleClick = async () => {
    await invoke("player_stop");
  };
  return (
    <PlaybackButton
      disabled={disabled}
      title="Stop"
      icon="Stop"
      onClick={handleClick}
    />
  );
}
