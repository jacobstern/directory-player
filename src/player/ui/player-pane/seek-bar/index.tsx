import { useEffect, useState } from "react";

import "./seek-bar.styles.css";
import { listen } from "@tauri-apps/api/event";
import { PlayerProgress, PlayerTrack } from "../../../types";

export default function SeekBar() {
  const [trackDuration, setTrackDuration] = useState<number | undefined>();
  const [progress, setProgress] = useState<number | undefined>();
  const value =
    trackDuration !== undefined && progress !== undefined
      ? progress / trackDuration
      : 0;
  useEffect(() => {
    let unlistenTrack: VoidFunction | undefined;
    let unlistenProgress: VoidFunction | undefined;
    (async () => {
      [unlistenTrack, unlistenProgress] = await Promise.all([
        listen<PlayerTrack>("player:track", (event) => {
          setTrackDuration(event.payload.duration);
        }),
        listen<PlayerProgress>("player:progress", (event) => {
          setProgress(event.payload);
        }),
      ]);
      return () => {
        unlistenTrack?.();
        unlistenProgress?.();
      };
    })();
  }, []);
  return <progress className="seek-bar" value={value} />;
}
