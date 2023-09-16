import { memo } from "react";
import "./RowListItem.css";
import ExpandButton from "./expand-button";
import classNames from "classnames";
import FileIcon from "./file-icon";
import { usePlaybackFile } from "../player";
import usePlaybackState from "../player/hooks/use-playback-state";

export interface RowListItemProps {
  path: string;
  name: string;
  type: "File" | "Directory";
  isExpanded?: boolean;
  canPlay?: boolean;
  depth: number;
  onExpandDirectory?: VoidFunction;
  onCollapseDirectory?: VoidFunction;
  onPlayback?: VoidFunction;
}

// TODO: Combine with TreeviewRow
function RowListItem({
  depth,
  name,
  type,
  path,
  canPlay,
  isExpanded,
  onExpandDirectory,
  onCollapseDirectory,
  onPlayback,
}: RowListItemProps) {
  const handleToggle = () => {
    if (isExpanded) {
      onCollapseDirectory?.();
    } else {
      onExpandDirectory?.();
    }
  };
  const handleDoubleClick: React.MouseEventHandler = (e) => {
    e.preventDefault();
    if (canPlay) {
      onPlayback?.();
    }
  };
  const playbackFile = usePlaybackFile();
  const playbackState = usePlaybackState();
  const isPlaying = playbackFile?.path === path;

  const nameClasses = classNames("row-list-item__name", {
    "row-list-item__name--no-left-indicator":
      type !== "Directory" && !isPlaying,
  });
  const firstColStyle: React.CSSProperties = {
    paddingLeft: depth * 8,
  };
  return (
    <li className="row-list-item" onDoubleClick={handleDoubleClick}>
      {isPlaying && (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          height="1em"
          viewBox="0 0 384 512"
          fill="currentColor"
          className={classNames("row-list-item__playing-indicator", {
            "row-list-item__playing-indicator--paused":
              playbackState === "Paused",
          })}
        >
          <title>Now playing</title>
          {/*! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc.*/}
          <path d="M160 80c0-26.5 21.5-48 48-48h32c26.5 0 48 21.5 48 48V432c0 26.5-21.5 48-48 48H208c-26.5 0-48-21.5-48-48V80zM0 272c0-26.5 21.5-48 48-48H80c26.5 0 48 21.5 48 48V432c0 26.5-21.5 48-48 48H48c-26.5 0-48-21.5-48-48V272zM368 96h32c26.5 0 48 21.5 48 48V432c0 26.5-21.5 48-48 48H368c-26.5 0-48-21.5-48-48V144c0-26.5 21.5-48 48-48z" />
        </svg>
      )}
      <div className="row-list-item__first-col" style={firstColStyle}>
        {type === "Directory" && (
          <ExpandButton isExpanded={isExpanded} onToggle={handleToggle} />
        )}
        <div className={nameClasses}>
          <FileIcon type={canPlay ? "MusicFile" : type} />
          {name}
        </div>
      </div>
    </li>
  );
}

export default memo(RowListItem);
