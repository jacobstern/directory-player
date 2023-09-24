import { useEffect, useState } from "react";
import "./volume-slider.styles.css";
import { invoke } from "@tauri-apps/api";
import syncStorage from "../../../../sync-storage";
import { z } from "zod";

const LOCAL_STORAGE_VOLUME_KEY = "volume";
const DEFAULT_VOLUME = 85;

function getPersistedVolumeOrDefault(): number {
  const value = syncStorage.getWithSchema(LOCAL_STORAGE_VOLUME_KEY, z.number());
  if (value === null) {
    return DEFAULT_VOLUME;
  }
  return value;
}

export default function VolumeSlider() {
  const [volume, setVolume] = useState(getPersistedVolumeOrDefault());
  useEffect(() => {
    invoke<void>("player_set_volume", { volume });
  }, [volume]);
  return (
    <input
      className="volume-slider"
      type="range"
      value={volume}
      onChange={(event) => {
        const volume = Number(event.target.value);
        syncStorage.set(LOCAL_STORAGE_VOLUME_KEY, volume);
        setVolume(volume);
      }}
      title="Volume"
    />
  );
}
