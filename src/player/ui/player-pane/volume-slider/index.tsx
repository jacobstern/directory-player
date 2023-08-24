import { ChangeEventHandler, useState } from "react";
import "./volume-slider.styles.css";

const LOCAL_STORAGE_VOLUME_KEY = "volume";
const DEFAULT_VOLUME = 85;

function getPersistedVolumeOrDefault(): number {
  const value = localStorage.getItem(LOCAL_STORAGE_VOLUME_KEY);
  if (!value) {
    return DEFAULT_VOLUME;
  }
  const parsed = Number.parseFloat(value);
  if (isNaN(parsed)) {
    return DEFAULT_VOLUME;
  }
  return parsed;
}

export default function VolumeSlider() {
  const [volume, setVolume] = useState(getPersistedVolumeOrDefault());
  const handleChange: ChangeEventHandler<HTMLInputElement> = (event) => {
    const stringValue = event.target.value;
    localStorage.setItem(LOCAL_STORAGE_VOLUME_KEY, stringValue);
    setVolume(Number(stringValue));
  };
  return (
    <input
      className="volume-slider"
      type="range"
      value={volume}
      onChange={handleChange}
    />
  );
}
