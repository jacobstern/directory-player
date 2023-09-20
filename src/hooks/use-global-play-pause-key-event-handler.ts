import { useEffect, useRef } from "react";

import { PlaybackState } from "../player/types";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { PlaybackStateSchema } from "../player/schemas";

export default function useGlobalPlayPauseKeyHandler() {
  const playbackStateRef = useRef<PlaybackState>("Stopped");
  useEffect(() => {
    let unlisten: VoidFunction | undefined;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === " ") {
        if (playbackStateRef.current === "Stopped") {
          return;
        }

        event.preventDefault();
        if (playbackStateRef.current === "Playing") {
          invoke("player_pause");
        } else if (playbackStateRef.current === "Paused") {
          invoke("player_play");
        }
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    async function setupListener() {
      unlisten = await listen("player://playback-state-change", (event) => {
        playbackStateRef.current = PlaybackStateSchema.parse(event.payload);
      });
    }
    setupListener();
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      unlisten?.();
    };
  }, []);
}
