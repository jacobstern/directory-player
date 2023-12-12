import classNames from "classnames";
import {
  MouseEventHandler,
  memo,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { FileType } from "../../../../core/file-type";
import ExpandButton from "./expand-button";
import FileIcon from "./file-icon";
import PlayingIndicator from "./playing-indicator";

import { showMenu } from "tauri-plugin-context-menu";

import "./row-list-item.styles.css";
import { invoke } from "@tauri-apps/api";
import {
  containsPathSeparator,
  renameLastSegment as replaceLastSegment,
} from "../../../../../utils/path";
import { renameFile } from "@tauri-apps/api/fs";

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
  const [isEditing, setIsEditing] = useState(false);
  const [optimisticName, setOptimisticName] = useState<string | null>(null);
  const nameInputRef = useRef<HTMLInputElement | null>(null);
  const didEnableEditingRef = useRef(false);
  const isRenamingRef = useRef(false);

  const currentName = optimisticName === null ? name : optimisticName;

  useLayoutEffect(() => {
    if (isEditing && didEnableEditingRef.current) {
      nameInputRef.current!.focus();
      let selectionEnd = currentName.length;
      if (fileType !== "directory" && currentName.includes(".")) {
        selectionEnd = currentName.lastIndexOf(".");
      }
      nameInputRef.current!.setSelectionRange(0, selectionEnd);
    }
    didEnableEditingRef.current = false;
  }, [fileType, isEditing, currentName]);

  const doRename = async () => {
    if (
      optimisticName === null ||
      optimisticName === "" ||
      containsPathSeparator(optimisticName)
    ) {
      setOptimisticName(null);
    } else {
      const newPath = replaceLastSegment(path, optimisticName);
      let success = true;
      isRenamingRef.current = true;
      try {
        await renameFile(path, newPath);
      } catch {
        success = false;
      }
      isRenamingRef.current = false;
      if (!success) {
        setOptimisticName(null);
      }
    }
  };

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
    showMenu({
      items: [
        // TODO: Open for non-music file
        {
          label: "Reveal in Finder",
          event: async () => {
            await invoke("show_in_folder", { path });
          },
        },
        {
          label: "Rename...",
          event: () => {
            didEnableEditingRef.current = true;
            setIsEditing(true);
          },
          disabled: isRenamingRef.current,
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
          {isEditing ? (
            <input
              className="row-list-item__name-input"
              ref={nameInputRef}
              type="text"
              value={currentName}
              onBlur={() => {
                setIsEditing(false);
                doRename();
              }}
              onKeyDown={(e) => {
                if (e.key === " ") {
                  e.stopPropagation();
                }
                if (e.key === "Enter") {
                  setIsEditing(false);
                  doRename();
                } else if (e.key === "Escape") {
                  setIsEditing(false);
                  setOptimisticName(null);
                }
              }}
              onChange={(e) => {
                setOptimisticName(e.target.value);
              }}
              onDoubleClick={(e) => {
                e.stopPropagation();
              }}
            ></input>
          ) : (
            currentName
          )}
        </div>
      </div>
    </li>
  );
});

export default RowListItem;
