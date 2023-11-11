import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import usePlaybackState from "../../../hooks/use-playback-state";

export default function PlayPauseButton() {
  const playbackState = usePlaybackState();
  const disabled = playbackState === "Stopped";
  const handleClick = async () => {
    if (playbackState === "Playing") {
      await invoke("player_pause");
    } else {
      await invoke("player_play");
    }
  };
  return (
    <PlaybackButton
      disabled={disabled}
      title={playbackState === "Playing" ? "Pause" : "Play"}
      icon={playbackState === "Playing" ? "pause" : "play"}
      onClick={handleClick}
    />
  );
}
