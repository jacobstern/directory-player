import { useCallback, useContext } from "react";
import useFlatListing from "../../hooks/use-flat-listing";
import TreeviewListing from "./treeview-listing";
import "./treeview-pane.styles.css";
import FileListingContext from "../../context/file-listing-context";
import ListingPlaceholder from "./listing-placeholder";

export default function TreeviewPane() {
  const fileListing = useContext(FileListingContext);
  if (fileListing === null) {
    throw new Error("File listing context must be initialized.");
  }
  const flatListing = useFlatListing(fileListing);

  const handleExpandDirectory = useCallback(
    (path: string) => {
      fileListing?.expandDirectory(path);
    },
    [fileListing],
  );
  const handleCollapseDirectory = useCallback(
    (path: string) => {
      fileListing?.collapseDirectory(path);
    },
    [fileListing],
  );

  return (
    <section className="treeview-pane">
      {flatListing === null ? (
        <ListingPlaceholder />
      ) : (
        <TreeviewListing
          flatListing={flatListing}
          onExpandDirectory={handleExpandDirectory}
          onCollapseDirectory={handleCollapseDirectory}
        />
      )}
    </section>
  );
}
