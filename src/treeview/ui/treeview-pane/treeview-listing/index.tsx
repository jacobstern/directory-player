import "./treeview-listing.styles.css";
import useFlatListing from "../../../hooks/use-flat-listing";
import ListingPlaceholder from "./listing-placeholder";
import RowListItem from "./row-list-item";
import { useCallback, useContext } from "react";
import FileListingContext from "../../../context/file-listing-context";

export default function TreeviewListing() {
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

  if (flatListing === null) {
    return <ListingPlaceholder />;
  }

  return (
    <div className="treeview-listing">
      <ol className="treeview-listing__container">
        {flatListing.map(({ path, name, fileType, depth, isExpanded }) => (
          <RowListItem
            key={path}
            path={path}
            name={name}
            fileType={fileType}
            depth={depth}
            isExpanded={isExpanded}
            onExpandDirectory={handleExpandDirectory}
            onCollapseDirectory={handleCollapseDirectory}
          />
        ))}
      </ol>
    </div>
  );
}
