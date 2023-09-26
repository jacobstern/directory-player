import { memo } from "react";
import usePlaybackState from "../../../../../../player/hooks/use-playback-state";
import classNames from "classnames";

import "./playing-indicator.styles.css";

const PlayingIndicator = memo(function PlayingIndicator() {
  const playbackState = usePlaybackState();
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      height="1em"
      viewBox="0 0 384 512"
      fill="currentColor"
      className={classNames("playing-indicator", {
        "playing-indicator--paused": playbackState === "Paused",
      })}
    >
      <title>Now playing</title>
      {/*! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc.*/}
      <path d="M160 80c0-26.5 21.5-48 48-48h32c26.5 0 48 21.5 48 48V432c0 26.5-21.5 48-48 48H208c-26.5 0-48-21.5-48-48V80zM0 272c0-26.5 21.5-48 48-48H80c26.5 0 48 21.5 48 48V432c0 26.5-21.5 48-48 48H48c-26.5 0-48-21.5-48-48V272zM368 96h32c26.5 0 48 21.5 48 48V432c0 26.5-21.5 48-48 48H368c-26.5 0-48-21.5-48-48V144c0-26.5 21.5-48 48-48z" />
    </svg>
  );
});

export default PlayingIndicator;
