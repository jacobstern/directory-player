import { z } from "zod";

export const PlaybackFileSchema = z.object({
  path: z.string(),
  name: z.string(),
});

export const PlaybackFileChangePayloadSchema = z.nullable(PlaybackFileSchema);

export const PlaybackStateSchema = z.enum(["Playing", "Paused", "Stopped"]);
