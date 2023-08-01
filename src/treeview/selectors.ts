import { AppState } from "../types";
import { NormalizedTreeviewItem } from "./types";
import { createSelector } from "reselect";

export type FlatListingItem = { path: string; depth: number };

function recursiveFlattenListing(
  listing: string[],
  directoryChildren: { [path: string]: string[] },
  items: { [path: string]: NormalizedTreeviewItem },
  depth: number,
): FlatListingItem[] {
  const sanitizedListing = listing
    .filter((path) => items[path] && !items[path].name.startsWith("."))
    .sort((a, b) => items[a].name.localeCompare(items[b].name));
  return sanitizedListing.flatMap((path) => {
    if (directoryChildren[path] && directoryChildren[path].length) {
      return [{ path, depth }].concat(
        recursiveFlattenListing(listing, directoryChildren, items, depth + 1),
      );
    }
    return { path, depth };
  });
}

export const selectFlatListing = createSelector(
  (state: AppState) => state.treeview.listing,
  (state: AppState) => state.treeview.directoryChildren,
  (state: AppState) => state.treeview.items,
  (listing, directoryChildren, items) => {
    return recursiveFlattenListing(listing, directoryChildren, items, 0);
  },
);
