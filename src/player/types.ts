import { z } from "zod";
import { PlaybackFileSchema } from "./schemas";

export interface PlayerTrack {
  path: number;
  duration: number;
}

export type PlayerProgress = number;

export type PlaybackFile = z.infer<typeof PlaybackFileSchema>;
