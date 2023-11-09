import {
  PointerEvent,
  PointerEventHandler,
  useEffect,
  useRef,
  useState,
} from "react";

import { StreamTiming } from "../../../types";
import classNames from "classnames";
import { invoke } from "@tauri-apps/api";

import "./seek-bar.styles.css";
import { StreamTimingChangePayloadSchema } from "../../../schemas";
import useEventListener from "../../../../tauri/hooks/use-event-listener";

/**
 * Grace period to get a new seek bar position from the server after a
 * seek action. During this time we will always display the requested
 * position.
 */
const OPTIMISTIC_POS_TIMEOUT_MILLISECONDS = 200;

function getNormalizedPosition(
  streamTiming: StreamTiming | null,
  optimisticPosition: number | undefined,
): number | undefined {
  if (streamTiming === null) return undefined;
  const position = optimisticPosition ?? streamTiming.pos;
  return position / streamTiming.duration;
}

function padDurationComponent(value: number): string {
  return String(value).padStart(2, "0");
}

function getDurationString(seconds: number | undefined): string | undefined {
  if (typeof seconds === "undefined") return undefined;
  const minutes = Math.floor(seconds / 60);
  const secondsComponent = Math.floor(seconds % 60);
  return [minutes, secondsComponent].map(padDurationComponent).join(":");
}

function getNormalizedOffset(e: PointerEvent): number {
  const clientRect = e.currentTarget.getBoundingClientRect();
  const offsetX = e.clientX - clientRect.left;
  const clampedOffset = Math.max(0, Math.min(offsetX, clientRect.width));
  return clampedOffset / clientRect.width;
}

export default function SeekBar() {
  const [streamTiming, setStreamTiming] = useState<StreamTiming | null>(null);
  const [optimisticPosition, setOptimisticPosition] = useState<
    number | undefined
  >();
  const [hoverOffset, setHoverOffset] = useState<number | undefined>();
  const isDraggingRef = useRef(false);
  const optimisticPosTimeoutRef = useRef<number | undefined>();
  const isMountedRef = useRef(false);
  const clientWidthRef = useRef<number | undefined>();
  const progressRef = useRef<HTMLProgressElement | null>(null);

  useEffect(() => {
    isMountedRef.current = true;
    clientWidthRef.current = progressRef.current!.getBoundingClientRect().width;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEventListener("player://stream-timing-change", (event) => {
    const payload = StreamTimingChangePayloadSchema.parse(event.payload);
    if (payload === null && optimisticPosition !== undefined) {
      setOptimisticPosition(undefined);
    }
    setStreamTiming(payload);
    console.log(payload?.duration, payload?.duration_seconds);
  });

  const handlePointerDown: PointerEventHandler = (e) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    isDraggingRef.current = true;
  };
  const handleLostPointerCapture: PointerEventHandler = () => {
    isDraggingRef.current = false;
    if (optimisticPosition !== undefined) {
      if (optimisticPosTimeoutRef.current !== undefined) {
        clearTimeout(optimisticPosTimeoutRef.current);
      }
      optimisticPosTimeoutRef.current = setTimeout(() => {
        if (isMountedRef.current) {
          setOptimisticPosition(undefined);
        }
      }, OPTIMISTIC_POS_TIMEOUT_MILLISECONDS);
    }
  };
  const handlePointerEnter: PointerEventHandler = (e) => {
    if (streamTiming !== null) {
      const normalizedOffset = getNormalizedOffset(e);
      setHoverOffset(normalizedOffset);
    }
  };
  const handlePointerMove: PointerEventHandler = (e) => {
    if (streamTiming !== null) {
      const normalizedOffset = getNormalizedOffset(e);
      setHoverOffset(normalizedOffset);
      if (e.currentTarget.hasPointerCapture(e.pointerId)) {
        setOptimisticPosition(normalizedOffset * streamTiming.duration);
      }
    }
  };
  const handlePointerUp: PointerEventHandler = (e) => {
    e.currentTarget.releasePointerCapture(e.pointerId);
    if (streamTiming !== null) {
      const seekPosition = getNormalizedOffset(e) * streamTiming.duration;
      setOptimisticPosition(seekPosition);
      invoke("player_seek", { offset: Math.floor(seekPosition) });
    }
  };

  const normalizedPosition = getNormalizedPosition(
    streamTiming,
    optimisticPosition,
  );
  const thumbTransform =
    normalizedPosition && clientWidthRef.current
      ? `translateX(${normalizedPosition * clientWidthRef.current}px)`
      : undefined;
  let seekTime: string | undefined;
  if (streamTiming !== null && hoverOffset !== undefined) {
    seekTime = getDurationString(hoverOffset * streamTiming.duration_seconds);
  }

  return (
    <div
      className={classNames("seek-bar", {
        "seek-bar--can-seek": streamTiming !== null,
      })}
      onPointerDown={handlePointerDown}
      onPointerEnter={handlePointerEnter}
      onPointerMove={handlePointerMove}
      onLostPointerCapture={handleLostPointerCapture}
      onPointerUp={handlePointerUp}
    >
      <progress
        className="seek-bar__progress"
        value={normalizedPosition}
        ref={progressRef}
      />
      {clientWidthRef.current !== undefined ? (
        <div
          className="seek-bar__seek-time"
          style={{
            left: clientWidthRef.current * (hoverOffset ?? 0),
          }}
        >
          {seekTime}
        </div>
      ) : undefined}
      <div
        className="seek-bar__thumb"
        style={{
          transform: thumbTransform,
        }}
      />
    </div>
  );
}
