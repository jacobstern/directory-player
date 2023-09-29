import { Event } from "@tauri-apps/api/event";
import "./stream-metadata.styles.css";
import { useMemo, useState } from "react";
import { StreamMetadataPayloadSchema } from "../../../schemas";
import type { StreamMetadata as StreamMetadataType } from "../../../types";
import useEventListener from "../../../../tauri/hooks/use-event-listener";
import debounce from "../../../../utils/debounce";
import classNames from "classnames";

const EVENT_LISTENER_DEBOUNCE_MILLIS = 300;

export default function StreamMetadata() {
  const [latestMetadata, setLatestMetadata] =
    useState<StreamMetadataType | null>(null);
  const [hasMetadata, setHasMetadata] = useState(false);
  const debouncedEventListener = useMemo(
    () =>
      debounce((e: Event<unknown>) => {
        const payload = StreamMetadataPayloadSchema.parse(e.payload);
        if (payload === null) {
          setHasMetadata(false);
        } else {
          setLatestMetadata(payload);
          setHasMetadata(true);
        }
      }, EVENT_LISTENER_DEBOUNCE_MILLIS),
    [],
  );
  useEventListener("player://stream-metadata-change", debouncedEventListener);
  const imageSrc = latestMetadata?.album_cover
    ? `data:${latestMetadata.album_cover.media_type};base64,${latestMetadata.album_cover.data_base64}`
    : undefined;
  return (
    <div
      className={classNames("stream-metadata", {
        "stream-metadata--has-metadata": hasMetadata,
      })}
    >
      {!!latestMetadata && (
        <>
          {imageSrc ? (
            <img
              alt="Album cover"
              className="stream-metadata__image"
              src={imageSrc}
            />
          ) : (
            <div
              role="presentation"
              className="stream-metadata__image-placeholder"
            />
          )}

          <div className="stream-metadata__title-and-artist">
            <div
              className="stream-metadata__title"
              title={latestMetadata.track_title ?? undefined}
            >
              {latestMetadata.track_title}
            </div>
            <div
              className="stream-metadata__artist"
              title={latestMetadata.artist ?? undefined}
            >
              {latestMetadata.artist}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
