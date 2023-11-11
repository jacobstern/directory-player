import PlayPauseButton from "./play-pause-button";
import SeekBar from "./seek-bar";
import SkipBackButton from "./skip-back-button";
import SkipForwardButton from "./skip-forward-button";
import VolumeSlider from "./volume-slider";

import "./player-pane.styles.css";
import StopButton from "./stop-button";
import StreamMetadata from "./stream-metadata";
import ShuffleButton from "./shuffle-button";

export default function PlayerPane() {
  return (
    <section className="player-pane" data-tauri-drag-region>
      <div className="player-pane__controls">
        <div className="player-pane__playback-buttons">
          <ShuffleButton />
          <SkipBackButton />
          <PlayPauseButton />
          <StopButton />
          <SkipForwardButton />
          <VolumeSlider />
        </div>
        <SeekBar />
      </div>
      <StreamMetadata />
    </section>
  );
}
