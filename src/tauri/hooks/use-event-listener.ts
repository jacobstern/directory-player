import { EventCallback, EventName, listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

export default function useEventListener(
  eventName: EventName,
  callback: EventCallback<unknown>,
): void {
  const callbackRef = useRef(callback);
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);
  useEffect(() => {
    let unlisten: VoidFunction | undefined;
    let needsDelayedCleanup = false;
    async function setupListener() {
      unlisten = await listen(eventName, (event) => {
        callbackRef.current(event);
      });
      if (needsDelayedCleanup) {
        unlisten();
      }
    }
    setupListener();
    return () => {
      unlisten?.();
      needsDelayedCleanup = true;
    };
  }, [eventName]);
}
