import { TreeviewView } from "./types";

export const ACTION_TREEVIEW_INIT = "treeview/TREEVIEW_INIT" as const;

export type TreeviewInitAction = {
  type: typeof ACTION_TREEVIEW_INIT;
  payload: TreeviewView;
};

export type TreeviewAction = TreeviewInitAction;
