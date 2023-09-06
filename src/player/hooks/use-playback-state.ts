import { useContext } from "react";

import { PlaybackStateContext } from "../context/playback-state-context";
import { PlaybackState } from "../types";

export default function usePlaybackState(): PlaybackState {
  return useContext(PlaybackStateContext);
}
