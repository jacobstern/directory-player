import { z } from "zod";
import { PlaybackFileSchema, PlaybackStateSchema } from "./schemas";

export interface PlayerTrack {
  path: number;
  duration: number;
}

export type PlayerProgress = number;

export type PlaybackFile = z.infer<typeof PlaybackFileSchema>;

export type PlaybackState = z.infer<typeof PlaybackStateSchema>;
