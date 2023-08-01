import { memo, useCallback, useMemo } from "react";
import { makeItemSelector } from "./selectors";
import { useSelector } from "react-redux";
import { AppState } from "../types";
import RowListItem from "./RowListItem";

export interface TreeviewRowProps {
  path: string;
  depth: number;
  onExpandDirectory: (path: string) => void;
  onCollapseDirectory: (path: string) => void;
}

function TreeviewRow({
  path,
  depth,
  onExpandDirectory,
  onCollapseDirectory,
}: TreeviewRowProps) {
  const selectItem = useMemo(makeItemSelector, []);
  const item = useSelector((state: AppState) => selectItem(state, path));

  const handleExpandDirectory = useCallback(() => {
    onExpandDirectory(path);
  }, [path, onExpandDirectory]);
  const handleCollapseDirectory = useCallback(() => {
    onCollapseDirectory(path);
  }, [path, onCollapseDirectory]);

  if (item.type === "Directory") {
    return (
      <RowListItem
        path={path}
        type="Directory"
        isExpanded={item.isExpanded}
        name={item.name}
        depth={depth}
        onExpandDirectory={handleExpandDirectory}
        onCollapseDirectory={handleCollapseDirectory}
      />
    );
  }
  return (
    <RowListItem
      path={path}
      type={item.type}
      name={item.name}
      depth={depth}
      onExpandDirectory={handleExpandDirectory}
      onCollapseDirectory={handleCollapseDirectory}
    />
  );
}

export default memo(TreeviewRow);
