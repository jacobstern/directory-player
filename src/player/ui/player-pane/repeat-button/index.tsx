import { invoke } from "@tauri-apps/api";

import PlaybackButton from "../../../shared-ui/playback-button";
import { useState } from "react";

type RepeatMode = "None" | "RepeatAll" | "RepeatOne";

function nextRepeatMode(repeatMode: RepeatMode): RepeatMode {
  switch (repeatMode) {
    case "None":
      return "RepeatAll";
    case "RepeatAll":
      return "RepeatOne";
    case "RepeatOne":
    default:
      return "None";
  }
}

export default function RepeatButton() {
  const [repeatMode, setRepeatMode] = useState<RepeatMode>("None");
  const desiredRepeatMode = nextRepeatMode(repeatMode);
  const handleClick = async () => {
    await invoke("player_set_repeat_mode", { repeatMode: desiredRepeatMode });
    setRepeatMode(desiredRepeatMode);
  };

  return (
    <PlaybackButton
      onClick={handleClick}
      selected={repeatMode !== "None"}
      title="Repeat"
      icon={repeatMode === "RepeatOne" ? "repeat-one" : "repeat"}
    />
  );
}
