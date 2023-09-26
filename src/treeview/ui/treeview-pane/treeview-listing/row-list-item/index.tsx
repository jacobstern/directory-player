import { memo } from "react";
import { FileType } from "../../../../core/file-type";
import classNames from "classnames";
import ExpandButton from "./expand-button";
import FileIcon from "./file-icon";

import "./row-list-item.styles.css";
import PlayingIndicator from "./playing-indicator";

export interface RowListItemProps {
  path: string;
  name: string;
  fileType: FileType;
  depth: number;
  isExpanded?: boolean;
}

const RowListItem = memo(function RowListItem({
  path,
  name,
  fileType,
  depth,
  isExpanded,
}: RowListItemProps) {
  const isPlaying = false;
  const firstColStyle: React.CSSProperties = {
    paddingLeft: depth * 8,
  };
  const handleToggle = () => {};

  const nameClasses = classNames("row-list-item__name", {
    "row-list-item__name--no-left-indicator":
      fileType !== "directory" && !isPlaying,
  });

  return (
    <li className="row-list-item">
      {isPlaying && <PlayingIndicator />}
      <div className="row-list-item__first-col" style={firstColStyle}>
        {fileType === "directory" && (
          <ExpandButton isExpanded={isExpanded} onToggle={handleToggle} />
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
