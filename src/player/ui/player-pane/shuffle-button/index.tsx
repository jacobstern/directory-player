import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import { useState } from "react";
import usePlaybackState from "../../../hooks/use-playback-state";

export default function ShuffleButton() {
  const [isShuffleEnabled, setIsShuffleEnabled] = useState(false);
  const handleClick = async () => {
    await invoke("player_set_shuffle_mode", {
      shuffleMode: isShuffleEnabled ? "NotEnabled" : "Enabled",
    });
    setIsShuffleEnabled(!isShuffleEnabled);
  };
  return (
    <PlaybackButton
      selected={isShuffleEnabled}
      title="Shuffle"
      icon="shuffle"
      onClick={handleClick}
    />
  );
}
