import { memo, useEffect } from "react";
import { useSelector } from "react-redux";
import { selectFlatListing } from "./selectors";

function TreeviewListing() {
  const flatListing = useSelector(selectFlatListing);
  useEffect(() => {
    console.log(flatListing);
  }, [flatListing]);
  return <div />;
}

export default memo(TreeviewListing);
