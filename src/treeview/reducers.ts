import { AppAction } from "../types";
import {
  ACTION_TREEVIEW_INIT,
  ACTION_TREEVIEW_ITEM_UPDATE,
} from "./actionTypes";
import {
  NormalizedDirectoryChildren,
  NormalizedItems,
  NormalizedTreeviewItem,
  TreeviewItem,
  TreeviewState,
} from "./types";
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
        case "Directory": {
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
        }
        default:
          return { 0: i.path, 1: i };
      }
    },
  );
}

function normalize(item: TreeviewItem): NormalizedTreeviewItem {
  if (item.type === "Directory") {
    return {
      type: "Directory",
      name: item.name,
      path: item.path,
      isExpanded: item.isExpanded,
    };
  }
  return item;
}

const itemsReducer: Reducer<NormalizedItems, AppAction> = (
  state = {},
  action,
) => {
  switch (action.type) {
    case ACTION_TREEVIEW_INIT:
      return Object.fromEntries(
        recursiveNormalize(action.payload.listing) as [
          [string, NormalizedTreeviewItem],
        ],
      );
    case ACTION_TREEVIEW_ITEM_UPDATE: {
      const updated = {
        ...state,
        [action.payload.path]: normalize(action.payload),
      };
      if (action.payload.type === "Directory") {
        Object.assign(
          updated,
          Object.fromEntries(
            recursiveNormalize(action.payload.children) as [
              // Typing for Object.fromEntries() is actually incorrect, it just
              // needsd to have "0" and "1" properties. Not using tuples avoids
              // any potential problem with Array.flatMap().
              [string, NormalizedTreeviewItem],
            ],
          ),
        );
      }
      return updated;
    }
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
      case "Directory": {
        const entry: NormalizedDirectoryChildrenEntry = {
          0: i.path,
          1: i.children.map((i) => i.path),
        };
        return [entry].concat(recursiveNormalizeChildren(i.children));
      }
      default:
        return [];
    }
  });
}

const directoryChildrenReducer: Reducer<
  NormalizedDirectoryChildren,
  AppAction
> = (state = {}, action) => {
  switch (action.type) {
    case ACTION_TREEVIEW_INIT:
      return Object.fromEntries(
        recursiveNormalizeChildren(action.payload.listing) as [
          [string, string[]],
        ],
      );
    case ACTION_TREEVIEW_ITEM_UPDATE:
      if (action.payload.type === "Directory") {
        return {
          ...state,
          [action.payload.path]: action.payload.children.map((i) => i.path),
        };
      }
      return state;
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
