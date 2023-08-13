import { useMemo } from "react";
import "./playback-button.styles.css";

export type ControlsButtonIcon =
  | "Play"
  | "Pause"
  | "SkipBack"
  | "SkipForward"
  | "Stop";

export interface ControlsButtonProps {
  icon: ControlsButtonIcon;
  disabled?: boolean;
  title: string;
  onClick?: VoidFunction;
}

export default function PlaybackButton({
  onClick,
  title,
  disabled,
  icon: namedIcon,
}: ControlsButtonProps) {
  const handleClick: React.MouseEventHandler = (e) => {
    e.preventDefault();
    onClick?.();
  };
  const icon = useMemo(() => {
    switch (namedIcon) {
      case "Play":
        return (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            height="1em"
            viewBox="0 0 384 512"
            fill="currentColor"
          >
            {/*! Font Awesome Free 6.4.2 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc. */}
            <path d="M73 39c-14.8-9.1-33.4-9.4-48.5-.9S0 62.6 0 80V432c0 17.4 9.4 33.4 24.5 41.9s33.7 8.1 48.5-.9L361 297c14.3-8.7 23-24.2 23-41s-8.7-32.2-23-41L73 39z" />
          </svg>
        );
      case "Pause":
        return (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            height="1em"
            viewBox="0 0 320 512"
            fill="currentColor"
          >
            {/*! Font Awesome Free 6.4.2 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc. */}
            <path d="M48 64C21.5 64 0 85.5 0 112V400c0 26.5 21.5 48 48 48H80c26.5 0 48-21.5 48-48V112c0-26.5-21.5-48-48-48H48zm192 0c-26.5 0-48 21.5-48 48V400c0 26.5 21.5 48 48 48h32c26.5 0 48-21.5 48-48V112c0-26.5-21.5-48-48-48H240z" />
          </svg>
        );
      default:
        return null;
    }
  }, [namedIcon]);
  return (
    <button
      type="button"
      disabled={disabled}
      title={title}
      onClick={handleClick}
      className="playback-button"
    >
      {icon}
    </button>
  );
}
