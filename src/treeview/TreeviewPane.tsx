import "./TreeviewPane.css";

import { memo } from "react";
import TreeviewListing from "./TreeviewListing";

function TreeviewPane() {
  return (
    <section className="treeview-pane">
      <TreeviewListing />
    </section>
  );
}

export default memo(TreeviewPane);
