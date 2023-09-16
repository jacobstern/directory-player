import { z } from "zod";
import {
  PlaybackFileSchema,
  PlaybackStateSchema,
  StreamTimingSchema,
} from "./schemas";

export type PlaybackFile = z.infer<typeof PlaybackFileSchema>;

export type PlaybackState = z.infer<typeof PlaybackStateSchema>;

export type StreamTiming = z.infer<typeof StreamTimingSchema>;
