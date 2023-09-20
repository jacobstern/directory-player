import { useEffect, useRef, useState } from "react";

import { StreamTiming } from "../../../types";
import classNames from "classnames";
import { invoke } from "@tauri-apps/api";

import "./seek-bar.styles.css";
import { StreamTimingChangePayloadSchema } from "../../../schemas";
import useEventListener from "../../../../tauri/hooks/use-event-listener";

/**
 * Grace period to get a new seek bar position from the server after a
 * seek action. During this time we will always display the requested
 * position. This also adds latency to the seek.
 */
const OPTIMISTIC_POS_TIMEOUT_MILLISECONDS = 200;

export default function SeekBar() {
  const [streamTiming, setStreamTiming] = useState<StreamTiming | null>(null);
  const [thumbPosition, setThumbPosition] = useState<number | undefined>();
  const [optimisticPos, setOptimisticPos] = useState<number | undefined>();
  const isDraggingRef = useRef(false);
  const optimisticPosTimeoutRef = useRef<number | undefined>();
  const isMountedRef = useRef(false);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEventListener("player://stream-timing-change", (event) => {
    const payload = StreamTimingChangePayloadSchema.parse(event.payload);
    if (payload === null && optimisticPos !== undefined) {
      setOptimisticPos(undefined);
    }
    setStreamTiming(payload);
  });

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
        if (optimisticPos !== undefined) {
          if (optimisticPosTimeoutRef.current !== undefined) {
            clearTimeout(optimisticPosTimeoutRef.current);
          }
          optimisticPosTimeoutRef.current = setTimeout(() => {
            if (isMountedRef.current) {
              setOptimisticPos(undefined);
            }
          }, OPTIMISTIC_POS_TIMEOUT_MILLISECONDS);
        }
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
          setOptimisticPos(offset);
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
