import { EventName } from "@tauri-apps/api/event";
import { useState } from "react";
import { z } from "zod";
import useEventListener from "./use-event-listener";

export default function useLatestEventPayload<T, D>(
  eventName: EventName,
  payloadSchema: z.Schema<T>,
  defaultValue: D,
): T | D {
  const [latestPayload, setLatestPayload] = useState<T | D>(defaultValue);
  useEventListener(eventName, (event) => {
    const { payload } = event;
    setLatestPayload(payloadSchema.parse(payload));
  });
  return latestPayload;
}
