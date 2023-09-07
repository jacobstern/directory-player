import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import usePlaybackState from "../../../hooks/use-playback-state";

export default function SkipBackButton() {
  const playbackState = usePlaybackState();
  const disabled = playbackState === "Stopped";
  const handleClick = async () => {
    await invoke("player_skip_back");
  };
  return (
    <PlaybackButton
      disabled={disabled}
      title="Skip Back"
      icon="SkipBack"
      onClick={handleClick}
    />
  );
}
