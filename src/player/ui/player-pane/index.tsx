import "./player-pane.styles.css";
import PauseButton from "./pause-button";
import PlayButton from "./play-button";
import VolumeSlider from "./volume-slider";

export default function PlayerPane() {
  return (
    <section className="player-pane">
      <div className="player-pane__controls">
        <div className="player-pane__playback-buttons">
          <PlayButton />
          <PauseButton />
          <VolumeSlider />
        </div>
      </div>
    </section>
  );
}
