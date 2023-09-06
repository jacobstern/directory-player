import { EventName, listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { z } from "zod";

export default function useLatestEventPayload<T, D>(
  event: EventName,
  payloadSchema: z.Schema<T>,
  defaultValue: D,
): T | D {
  const [latestPayload, setLatestPayload] = useState<T | D>(defaultValue);
  useEffect(() => {
    let unlisten: VoidFunction | undefined;
    async function setupListener() {
      unlisten = await listen(event, (event) => {
        const { payload } = event;
        setLatestPayload(payloadSchema.parse(payload));
      });
    }
    setupListener();
    return () => {
      unlisten?.();
    };
  }, [defaultValue, event, payloadSchema]);
  return latestPayload;
}
