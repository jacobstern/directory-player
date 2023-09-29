import { z } from "zod";

export const PlaybackFileSchema = z.object({
  path: z.string(),
  name: z.string(),
});

export const PlaybackFileChangePayloadSchema = z.nullable(PlaybackFileSchema);

export const PlaybackStateSchema = z.enum(["Playing", "Paused", "Stopped"]);

export const StreamTimingSchema = z.object({
  duration: z.number(),
  pos: z.number(),
  duration_seconds: z.number(),
});

export const StreamTimingChangePayloadSchema = z.nullable(StreamTimingSchema);

export const StreamMetadataSchema = z.object({
  track_title: z.string().nullable(),
  artist: z.string().nullable(),
  album_cover_base64: z.string().nullable(),
});

export const StreamMetadataPayloadSchema = z.nullable(StreamMetadataSchema);
