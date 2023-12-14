import { Event } from "@tauri-apps/api/event";
import "./stream-metadata.styles.css";
import { useMemo, useState } from "react";
import { StreamMetadataPayloadSchema } from "../../../schemas";
import type { StreamMetadata } from "../../../types";
import useEventListener from "../../../../tauri/hooks/use-event-listener";
import debounce from "../../../../utils/debounce";
import classNames from "classnames";
import { usePlaybackFile } from "../../..";

const EVENT_LISTENER_DEBOUNCE_MILLIS = 100;

function stripExtension(
  playbackFilePath: string | undefined,
): string | undefined {
  if (typeof playbackFilePath === "undefined") return undefined;
  const dotIndex = playbackFilePath.lastIndexOf(".");
  if (dotIndex >= 0) {
    return playbackFilePath.substring(0, dotIndex);
  }
  return playbackFilePath;
}

export default function StreamMetadata() {
  const playbackFileName = usePlaybackFile()?.name;
  const [latestMetadata, setLatestMetadata] = useState<StreamMetadata | null>(
    null,
  );
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
      <>
        {imageSrc ? (
          <img
            alt="Album cover"
            className="stream-metadata__image"
            src={imageSrc}
          />
        ) : (
          !!latestMetadata && (
            <div
              role="presentation"
              className="stream-metadata__image-placeholder"
            />
          )
        )}

        <div className="stream-metadata__second-column">
          <div className="stream-metadata__title">
            {latestMetadata?.track_title ?? stripExtension(playbackFileName)}
          </div>
          <div className="stream-metadata__artist">
            {latestMetadata?.artist}
          </div>
        </div>
      </>
    </div>
  );
}
