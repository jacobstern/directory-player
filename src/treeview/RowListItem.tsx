import { memo } from "react";
import "./RowListItem.css";
import ExpandButton from "./ExpandButton";
import classNames from "classnames";

export interface RowListItemProps {
  path: string;
  name: string;
  type: "File" | "Directory";
  isExpanded?: boolean;
  depth: number;
  onExpandDirectory: VoidFunction;
  onCollapseDirectory: VoidFunction;
}

function RowListItem({
  depth,
  name,
  type,
  isExpanded,
  onExpandDirectory,
  onCollapseDirectory,
}: RowListItemProps) {
  const handleToggle = () => {
    if (isExpanded) {
      onCollapseDirectory();
    } else {
      onExpandDirectory();
    }
  };

  const nameClasses = classNames("row-list-item__name", {
    "row-list-item__name--no-directory": type !== "Directory",
  });
  const firstColStyle: React.CSSProperties = {
    paddingLeft: depth * 6,
  };
  return (
    <li className="row-list-item">
      <div className="row-list-item__first-col" style={firstColStyle}>
        {type === "Directory" && (
          <ExpandButton isExpanded={isExpanded} onToggle={handleToggle} />
        )}
        <div className={nameClasses}>{name}</div>
      </div>
    </li>
  );
}

export default memo(RowListItem);
