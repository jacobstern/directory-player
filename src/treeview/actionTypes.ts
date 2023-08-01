import { TreeviewItem, TreeviewView } from "./types";

export const ACTION_TREEVIEW_INIT = "treeview/INIT" as const;
export const ACTION_TREEVIEW_ITEM_UPDATE = "treeview/ITEM_UPDATE" as const;

export type TreeviewInitAction = {
  type: typeof ACTION_TREEVIEW_INIT;
  payload: TreeviewView;
};

export type TreeviewItemUpdateAction = {
  type: typeof ACTION_TREEVIEW_ITEM_UPDATE;
  payload: TreeviewItem;
};

export type TreeviewAction = TreeviewInitAction | TreeviewItemUpdateAction;
