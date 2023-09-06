import { PlaybackFileChangePayloadSchema } from "../schemas";
import { PlaybackFile } from "../types";
import { useLatestEventPayload } from "../../tauri";

export default function usePlaybackFile(): PlaybackFile | null {
  return useLatestEventPayload(
    "player:playbackFileChange",
    PlaybackFileChangePayloadSchema,
    null,
  );
}
