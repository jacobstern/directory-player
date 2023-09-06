import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { PlaybackFileSchema } from "../schemas";
import { PlaybackFile } from "../types";

export default function usePlaybackFile(): PlaybackFile | null {
  const [playbackFile, setPlaybackFile] = useState<PlaybackFile | null>(null);
  useEffect(() => {
    let unlisten: VoidFunction | undefined;
    async function subscribe() {
      unlisten = await listen("player:playbackFileChange", (event) => {
        const { payload } = event;
        console.log("Received playback file change", payload);
        const playbackFile = PlaybackFileSchema.nullable().parse(payload);
        setPlaybackFile(playbackFile);
      });
    }
    subscribe();
    return () => {
      unlisten?.();
    };
  }, []);
  return playbackFile;
}
