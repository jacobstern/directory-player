import "./player-pane.styles.css";
import VolumeSlider from "./volume-slider";
import SeekBar from "./seek-bar";
import PlayPauseButton from "./play-pause-button";

export default function PlayerPane() {
  return (
    <section className="player-pane">
      <div className="player-pane__controls">
        <div className="player-pane__playback-buttons">
          <PlayPauseButton />
          <VolumeSlider />
        </div>
        <SeekBar />
      </div>
    </section>
  );
}
