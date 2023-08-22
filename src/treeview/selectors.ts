import { AppState } from "../types";
import { NormalizedTreeviewItem } from "./types";
import { createSelector } from "reselect";

export type FlatListingItem = { path: string; depth: number };

export const makeItemSelector = () => {
  const selectItem = createSelector(
    [(state: AppState) => state.treeview.items, (_state, path: string) => path],
    (items, path) => items[path]!,
  );
  return selectItem;
};

function recursiveFlattenListing(
  listing: string[],
  directoryChildren: { [path: string]: string[] },
  items: { [path: string]: NormalizedTreeviewItem },
  depth: number,
): FlatListingItem[] {
  return listing.flatMap((path) => {
    const children = directoryChildren[path];
    if (children?.length) {
      return [{ path, depth }].concat(
        recursiveFlattenListing(
          children,
          directoryChildren,
          items,
          depth + 1,
        ),
      );
    }
    return { path, depth };
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
