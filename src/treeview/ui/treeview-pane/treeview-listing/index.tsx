import "./treeview-listing.styles.css";
import useFlatListing from "../../../hooks/use-flat-listing";
import ListingPlaceholder from "./listing-placeholder";
import RowListItem from "./row-list-item";

export default function TreeviewListing() {
  const flatListing = useFlatListing();

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
          />
        ))}
      </ol>
    </div>
  );
}
