import classNames from "classnames";
import { MouseEventHandler, memo } from "react";
import { FileType } from "../../../../core/file-type";
import ExpandButton from "./expand-button";
import FileIcon from "./file-icon";
import PlayingIndicator from "./playing-indicator";

import { open } from "@tauri-apps/api/shell";
import { showMenu } from "tauri-plugin-context-menu";
import { getParentPath } from "../../../../../utils/path";

import "./row-list-item.styles.css";

export interface RowListItemProps {
  path: string;
  name: string;
  fileType: FileType;
  depth: number;
  isExpanded?: boolean;
  isPlaying: boolean;
  onExpandDirectory: (path: string) => void;
  onCollapseDirectory: (path: string) => void;
  onPlayback: (path: string) => void;
}

const RowListItem = memo(function RowListItem({
  path,
  name,
  fileType,
  depth,
  isExpanded,
  isPlaying,
  onExpandDirectory,
  onCollapseDirectory,
  onPlayback: onPlay,
}: RowListItemProps) {
  const firstColStyle: React.CSSProperties = {
    paddingLeft: depth * 8,
  };
  const toggleExpanded = () => {
    if (isExpanded) {
      onCollapseDirectory(path);
    } else {
      onExpandDirectory(path);
    }
  };
  const handleDoubleClick = () => {
    if (fileType === "music-file") {
      onPlay(path);
    } else if (fileType === "directory") {
      toggleExpanded();
    }
  };
  const handleContextMenu: MouseEventHandler = (event) => {
    event.preventDefault();
    const directoryPath = fileType === "directory" ? path : getParentPath(path);
    showMenu({
      items: [
        {
          label: "Open Folder",
          event: async () => {
            await open(directoryPath);
          },
        },
      ],
    });
  };

  const nameClasses = classNames("row-list-item__name", {
    "row-list-item__name--no-left-indicator":
      fileType !== "directory" && !isPlaying,
  });

  return (
    <li
      className="row-list-item"
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
    >
      {isPlaying && <PlayingIndicator />}
      <div className="row-list-item__first-col" style={firstColStyle}>
        {fileType === "directory" && (
          <ExpandButton isExpanded={isExpanded} onToggle={toggleExpanded} />
        )}
        <div className={nameClasses}>
          <FileIcon fileType={fileType} />
          {name}
        </div>
      </div>
    </li>
  );
});

export default RowListItem;
