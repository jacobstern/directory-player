import usePlaybackFile from "../../player/hooks/use-playback-file";
import "./playing-indicator.styles.css";

export default function PlayingIndicator() {
  const playbackFile = usePlaybackFile();
  if (!playbackFile) return null;
  return <div className="playing-indicator">{playbackFile.name}</div>;
}
