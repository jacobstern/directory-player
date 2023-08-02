import { AppState } from "../types";
import { NormalizedTreeviewItem } from "./types";
import { createSelector } from "reselect";

export type FlatListingItem = { path: string; depth: number; canPlay: boolean };

export const makeItemSelector = () => {
  const selectItem = createSelector(
    [(state: AppState) => state.treeview.items, (_state, path: string) => path],
    (items, path) => items[path],
  );
  return selectItem;
};

function recursiveFlattenListing(
  listing: string[],
  directoryChildren: { [path: string]: string[] },
  items: { [path: string]: NormalizedTreeviewItem },
  depth: number,
): FlatListingItem[] {
  // TODO: Filter and sort on the server
  const sanitizedListing = listing
    .filter((path) => items[path] && !items[path].name.startsWith("."))
    .sort((a, b) => items[a].name.localeCompare(items[b].name));
  return sanitizedListing.flatMap((path) => {
    let canPlay = false;
    const item = items[path];
    if (item && item.type === "File") {
      canPlay = item.canPlay;
    }
    if (directoryChildren[path] && directoryChildren[path].length) {
      return [{ path, depth, canPlay }].concat(
        recursiveFlattenListing(
          directoryChildren[path],
          directoryChildren,
          items,
          depth + 1,
        ),
      );
    }
    return { path, depth, canPlay };
  });
}

export const selectFlatListing = createSelector(
  [
    (state: AppState) => state.treeview.listing,
    (state: AppState) => state.treeview.directoryChildren,
    (state: AppState) => state.treeview.items,
  ],
  (listing, directoryChildren, items) => {
    return recursiveFlattenListing(listing, directoryChildren, items, 0);
  },
);
