import { memo, useEffect, useState } from "react";
import "./player-debug-overlay.styles.css";
import { listen } from "@tauri-apps/api/event";
import { PlayerProgress } from "../../types";

export default memo(function PlayerDebugOverlay() {
  const [progress, setProgress] = useState(0);
  useEffect(() => {
    let unlistenProgress: VoidFunction | undefined;
    (async () => {
      listen<PlayerProgress>("player://progress", (event) => {
        setProgress(event.payload);
      });
    })();
    return () => {
      unlistenProgress?.();
    };
  }, []);
  return <div className="debug-overlay">Progress: {progress}</div>;
});
