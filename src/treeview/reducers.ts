import { AppAction } from "../types";
import { ACTION_TREEVIEW_INIT } from "./actionTypes";
import { NormalizedTreeviewItem, TreeviewItem, TreeviewState } from "./types";
import { combineReducers } from "redux";
import type { Reducer } from "redux";

const listingReducer: Reducer<string[], AppAction> = (state = [], action) => {
  switch (action.type) {
    case ACTION_TREEVIEW_INIT:
      return action.payload.listing.map((i) => i.path);
    default:
      return state;
  }
};

type NormalizedTreeviewEntry = { 0: string; 1: NormalizedTreeviewItem };

function recursiveNormalize(items: TreeviewItem[]): NormalizedTreeviewEntry[] {
  return items.flatMap(
    (i: TreeviewItem): NormalizedTreeviewEntry | NormalizedTreeviewEntry[] => {
      switch (i.type) {
        case "Directory":
          const entry: NormalizedTreeviewEntry = {
            0: i.path,
            1: {
              type: "Directory",
              path: i.path,
              name: i.name,
              isExpanded: i.isExpanded,
            },
          };
          return [entry].concat(recursiveNormalize(i.children));
        default:
          return { 0: i.path, 1: i };
      }
    },
  );
}

const itemsReducer: Reducer<
  { [path: string]: NormalizedTreeviewItem },
  AppAction
> = (state = {}, action) => {
  switch (action.type) {
    case ACTION_TREEVIEW_INIT:
      return Object.fromEntries(
        recursiveNormalize(action.payload.listing) as [
          [string, NormalizedTreeviewItem],
        ],
      );
    default:
      return state;
  }
};

type NormalizedDirectoryChildrenEntry = { 0: string; 1: string[] };

function recursiveNormalizeChildren(
  items: TreeviewItem[],
): NormalizedDirectoryChildrenEntry[] {
  return items.flatMap((i) => {
    switch (i.type) {
      case "Directory":
        const entry: NormalizedDirectoryChildrenEntry = {
          0: i.path,
          1: i.children.map((i) => i.path),
        };
        return [entry].concat(recursiveNormalizeChildren(i.children));
      default:
        return [];
    }
  });
}

const directoryChildrenReducer: Reducer<
  { [path: string]: string[] },
  AppAction
> = (state = {}, action) => {
  switch (action.type) {
    case ACTION_TREEVIEW_INIT:
      return Object.fromEntries(
        recursiveNormalizeChildren(action.payload.listing) as [
          [string, string[]],
        ],
      );
    default:
      return state;
  }
};

export const treeviewReducer: Reducer<TreeviewState, AppAction> =
  combineReducers({
    listing: listingReducer,
    items: itemsReducer,
    directoryChildren: directoryChildrenReducer,
  });
