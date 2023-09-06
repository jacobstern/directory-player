import { useContext } from "react";

import { PlaybackFileContext } from "../context/playback-file-context";
import { PlaybackFile } from "../types";

export default function usePlaybackFile(): PlaybackFile | null {
  return useContext(PlaybackFileContext);
}
