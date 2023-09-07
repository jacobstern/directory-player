import PlayPauseButton from "./play-pause-button";
import SeekBar from "./seek-bar";
import SkipForwardButton from "./skip-forward-button";
import VolumeSlider from "./volume-slider";

import "./player-pane.styles.css";

export default function PlayerPane() {
  return (
    <section className="player-pane">
      <div className="player-pane__controls">
        <div className="player-pane__playback-buttons">
          <PlayPauseButton />
          <SkipForwardButton />
          <VolumeSlider />
        </div>
        <SeekBar />
      </div>
    </section>
  );
}
