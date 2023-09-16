import { useEffect, useRef, useState } from "react";

import { listen } from "@tauri-apps/api/event";
import { StreamTiming } from "../../../types";
import classNames from "classnames";
import { invoke } from "@tauri-apps/api";

import "./seek-bar.styles.css";
import { StreamTimingChangePayloadSchema } from "../../../schemas";

export default function SeekBar() {
  const [streamTiming, setStreamTiming] = useState<StreamTiming | null>(null);
  const [thumbPosition, setThumbPosition] = useState<number | undefined>();
  const [optimisticPos, setOptimisticPos] = useState<number | undefined>();
  const isDraggingRef = useRef(false);

  useEffect(() => {
    let unlistenProgress: VoidFunction | undefined;
    (async () => {
      unlistenProgress = await listen("player:streamTimingChange", (event) => {
        setStreamTiming(StreamTimingChangePayloadSchema.parse(event.payload));
        if (!isDraggingRef.current) {
          setOptimisticPos(undefined);
        }
      });
      return () => {
        unlistenProgress?.();
      };
    })();
  }, []);

  return (
    <div
      className={classNames("seek-bar", {
        "seek-bar--can-seek": streamTiming !== null,
      })}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        isDraggingRef.current = true;
      }}
      onPointerMove={(e) => {
        const clientRect = e.currentTarget.getBoundingClientRect();
        const offsetX = e.clientX - clientRect.left;
        const clampedOffset = Math.max(0, Math.min(offsetX, clientRect.width));
        setThumbPosition(clampedOffset);
        if (
          e.currentTarget.hasPointerCapture(e.pointerId) &&
          streamTiming !== null
        ) {
          setOptimisticPos(
            (clampedOffset * streamTiming.duration) / clientRect.width,
          );
        }
      }}
      onLostPointerCapture={() => {
        isDraggingRef.current = false;
      }}
      onPointerUp={(e) => {
        e.currentTarget.releasePointerCapture(e.pointerId);
        if (streamTiming !== null) {
          const clientRect = e.currentTarget.getBoundingClientRect();
          const offsetX = e.clientX - clientRect.left;
          const normalizedPos = Math.max(
            0,
            Math.min(1, offsetX / clientRect.width),
          );
          const offset = Math.floor(normalizedPos * streamTiming.duration);
          invoke("player_seek", { offset });
        }
      }}
    >
      <progress
        className="seek-bar__progress"
        value={
          streamTiming !== null
            ? (optimisticPos ?? streamTiming.pos) / streamTiming.duration
            : undefined
        }
      />
      <div
        className={classNames("seek-bar__thumb", {
          "seek-bar__thumb--has-position": thumbPosition !== undefined,
        })}
        style={{
          transform: thumbPosition
            ? `translateX(${thumbPosition}px)`
            : undefined,
        }}
      />
    </div>
  );
}
