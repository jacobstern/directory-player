import { ReactNode, createContext } from "react";

import { PlaybackState } from "../types";
import { useLatestEventPayload } from "../../tauri";
import { PlaybackStateSchema } from "../schemas";

export const PlaybackStateContext = createContext<PlaybackState>("Stopped");

export interface PlaybackStateProviderProps {
  children?: ReactNode;
}

export function PlaybackStateProvider({
  children,
}: PlaybackStateProviderProps) {
  const value = useLatestEventPayload(
    "player://playback-state-change",
    PlaybackStateSchema,
    "Stopped",
  );
  return (
    <PlaybackStateContext.Provider value={value}>
      {children}
    </PlaybackStateContext.Provider>
  );
}
