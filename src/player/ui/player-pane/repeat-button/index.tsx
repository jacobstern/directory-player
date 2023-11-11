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
  const handleClick = () => {
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
