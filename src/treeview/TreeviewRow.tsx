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
  onPlayback: (path: string) => void;
}

function TreeviewRow({
  path,
  depth,
  onExpandDirectory,
  onCollapseDirectory,
  onPlayback,
}: TreeviewRowProps) {
  const selectItem = useMemo(makeItemSelector, []);
  const item = useSelector((state: AppState) => selectItem(state, path));

  const handleExpandDirectory = useCallback(() => {
    onExpandDirectory(path);
  }, [path, onExpandDirectory]);
  const handleCollapseDirectory = useCallback(() => {
    onCollapseDirectory(path);
  }, [path, onCollapseDirectory]);
  const handlePlayback = useCallback(() => {
    onPlayback(path);
  }, [path, onPlayback]);

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
        onPlayback={handlePlayback}
      />
    );
  }
  return (
    <RowListItem
      path={path}
      type={item.type}
      name={item.name}
      depth={depth}
      canPlay={item.canPlay}
      onPlayback={handlePlayback}
    />
  );
}

export default memo(TreeviewRow);
