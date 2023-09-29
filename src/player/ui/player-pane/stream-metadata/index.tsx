import { Event } from "@tauri-apps/api/event";
import "./stream-metadata.styles.css";
import { useMemo, useState } from "react";
import { StreamMetadataPayloadSchema } from "../../../schemas";
import type { StreamMetadata as StreamMetadataType } from "../../../types";
import useEventListener from "../../../../tauri/hooks/use-event-listener";
import debounce from "../../../../utils/debounce";

const EVENT_LISTENER_DEBOUNCE_MILLIS = 300;

export default function StreamMetadata() {
  const [latestMetadata, setLatestMetadata] =
    useState<StreamMetadataType | null>(null);
  const debouncedEventListener = useMemo(
    () =>
      debounce((e: Event<unknown>) => {
        const payload = StreamMetadataPayloadSchema.parse(e.payload);
        if (payload !== null) {
          setLatestMetadata(payload);
        }
      }, EVENT_LISTENER_DEBOUNCE_MILLIS),
    [],
  );
  useEventListener("player://stream-metadata-change", debouncedEventListener);
  return (
    <div className="stream-metadata">
      {latestMetadata && (
        <>
          <div>{latestMetadata.track_title}</div>
          <div>{latestMetadata.artist}</div>
        </>
      )}
    </div>
  );
}
