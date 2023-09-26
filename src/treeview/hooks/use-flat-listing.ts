import { useCallback, useContext, useEffect, useState } from "react";

import FileListingContext from "../context/file-listing-context";
import { FlatListing, generateFullListing } from "../core/flat-listing";
import { ChangeListener } from "../core/file-listing";

export default function useFlatListing(): FlatListing {
  const fileListing = useContext(FileListingContext);
  if (fileListing === null) {
    throw new Error("File listing context must be initialized.");
  }

  const initialListing = generateFullListing(fileListing.getRoot());
  const [flatListing, setFlatListing] = useState<FlatListing>(initialListing);

  const handleListingChange: ChangeListener = useCallback(() => {
    const updatedListing = generateFullListing(fileListing.getRoot());
    setFlatListing(updatedListing);
  }, [fileListing]);

  useEffect(() => {
    const unlisten = fileListing.addChangeListener(handleListingChange);
    return () => {
      unlisten();
    };
  }, [fileListing, handleListingChange]);

  return flatListing;
}
