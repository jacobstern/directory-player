import "./TreeviewPane.css";

import TreeviewListing from "./TreeviewListing";
import PlayingIndicator from "./playing-indicator";

export default function TreeviewPane() {
  return (
    <section className="treeview-pane">
      <TreeviewListing />
      <PlayingIndicator />
    </section>
  );
}
