import "./listing-placeholder.styles.css";

export default function ListingPlaceholder() {
  return (
    <div className="listing-placeholder">
      Please use{" "}
      <pre className="listing-placeholder__verbatim">
        File &gt; Open Folder...
      </pre>{" "}
      or <pre className="listing-placeholder__verbatim">⌘O</pre> to select a
      library folder.
    </div>
  );
}
