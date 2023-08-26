import { CSSProperties, MouseEventHandler, useEffect, useState } from "react";

import "./seek-bar.styles.css";
import { listen } from "@tauri-apps/api/event";
import { PlayerProgress, PlayerTrack } from "../../../types";
import classNames from "classnames";
import { invoke } from "@tauri-apps/api";

const VALUE_FUDGE_FACTOR = 0.015;

export default function SeekBar() {
  const [trackDuration, setTrackDuration] = useState<number | undefined>();
  const [progress, setProgress] = useState<number | undefined>();
  const [mouseLeft, setMouseLeft] = useState<number | undefined>();

  useEffect(() => {
    let unlistenTrack: VoidFunction | undefined;
    let unlistenProgress: VoidFunction | undefined;
    (async () => {
      [unlistenTrack, unlistenProgress] = await Promise.all([
        listen<PlayerTrack>("player:track", (event) => {
          setTrackDuration(event.payload.duration);
          // Progress is probably invalid, otherwise we will get a new progress message soon
          setProgress(undefined);
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

  const value =
    trackDuration !== undefined && progress !== undefined
      ? VALUE_FUDGE_FACTOR + progress / trackDuration
      : 0;

  const canSeek = trackDuration !== undefined && progress !== undefined;
  const shouldShowThumb = canSeek && mouseLeft !== undefined;
  const thumbStyle: CSSProperties = {
    transform: `translateX(${mouseLeft}px)`,
    opacity: shouldShowThumb ? 1 : 0,
  };

  const mouseEnterOrMoveHandler: MouseEventHandler<HTMLDivElement> = (e) => {
    setMouseLeft(
      Math.max(e.clientX - e.currentTarget.getBoundingClientRect().left),
    );
  };
  const mouseLeaveHandler: MouseEventHandler<HTMLDivElement> = () => {
    setMouseLeft(undefined);
  };
  const clickHandler: MouseEventHandler<HTMLDivElement> = (e) => {
    if (canSeek && trackDuration !== undefined) {
      const clientRect = e.currentTarget.getBoundingClientRect();
      const left = e.clientX - clientRect.left;
      const normalizedPos = Math.max(0, Math.min(1, left / clientRect.width));
      const offset = Math.floor(normalizedPos * trackDuration);
      invoke("player_seek", { offset });
    }
  };

  return (
    <div
      className={classNames("seek-bar", { "seek-bar--can-seek": canSeek })}
      onClick={clickHandler}
      onMouseEnter={mouseEnterOrMoveHandler}
      onMouseMove={mouseEnterOrMoveHandler}
      onMouseLeave={mouseLeaveHandler}
    >
      <progress className="seek-bar__progress" value={value} />
      <div className="seek-bar__thumb" style={thumbStyle} />
    </div>
  );
}
