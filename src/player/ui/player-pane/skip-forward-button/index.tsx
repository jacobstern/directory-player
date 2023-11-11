import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import usePlaybackState from "../../../hooks/use-playback-state";

export default function SkipForwardButton() {
  const playbackState = usePlaybackState();
  const disabled = playbackState === "Stopped";
  const handleClick = async () => {
    await invoke("player_skip_forward");
  };
  return (
    <PlaybackButton
      disabled={disabled}
      title="Skip Forward"
      icon="skip-forward"
      onClick={handleClick}
    />
  );
}
