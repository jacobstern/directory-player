import { createContext, ReactNode } from "react";

import { useLatestEventPayload } from "../../tauri";
import { PlaybackFileChangePayloadSchema } from "../schemas";
import { PlaybackFile } from "../types";

export const PlaybackFileContext = createContext<PlaybackFile | null>(null);

export interface PlaybackFileProviderProps {
  children?: ReactNode;
}

export function PlaybackFileProvider({ children }: PlaybackFileProviderProps) {
  const value = useLatestEventPayload(
    "player://playback-file-change",
    PlaybackFileChangePayloadSchema,
    null,
  );
  return (
    <PlaybackFileContext.Provider value={value}>
      {children}
    </PlaybackFileContext.Provider>
  );
}
