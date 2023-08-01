import { memo, useCallback } from "react";
import { useDispatch, useSelector } from "react-redux";

import "./TreeviewListing.css";
import { selectFlatListing } from "./selectors";
import TreeviewRow from "./TreeviewRow";
import { treeviewItemUpdate } from "./actions";
import { TreeviewItem } from "./types";
import { invoke } from "@tauri-apps/api";

function TreeviewListing() {
  const dispatch = useDispatch();
  const flatListing = useSelector(selectFlatListing);
  const handleExpandDirectory = useCallback(
    (path: string) => {
      invoke("treeview_expand_directory", { clientPath: path }).then(
        (result) => {
          dispatch(treeviewItemUpdate(result as TreeviewItem));
          console.log(result);
        },
      );
    },
    [dispatch],
  );
  const handleCollapseDirectory = useCallback(
    (path: string) => {
      invoke("treeview_collapse_directory", { clientPath: path }).then(
        (result) => {
          dispatch(treeviewItemUpdate(result as TreeviewItem));
          console.log(result);
        },
      );
    },
    [dispatch],
  );
  return (
    <div className="treeview-listing">
      <ol className="treeview-listing__container">
        {flatListing.map(({ path, depth }) => (
          <TreeviewRow
            key={path}
            path={path}
            depth={depth}
            onExpandDirectory={handleExpandDirectory}
            onCollapseDirectory={handleCollapseDirectory}
          />
        ))}
      </ol>
    </div>
  );
}

export default memo(TreeviewListing);
