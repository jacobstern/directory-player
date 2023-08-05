import { TreeviewAction } from "./treeview/actionTypes";
import { TreeviewState } from "./treeview/types";

export type AppAction = TreeviewAction;

export type AppState = {
  treeview: TreeviewState;
};
