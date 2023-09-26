import { useCallback, useEffect, useState } from "react";

import { FlatListing, generateFullListing } from "../core/flat-listing";
import { ChangeListener, FileListing } from "../core/file-listing";

export default function useFlatListing(fileListing: FileListing): FlatListing {
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
