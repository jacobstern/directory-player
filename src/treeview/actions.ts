import { ACTION_TREEVIEW_INIT, TreeviewInitAction } from "./actionTypes";
import { TreeviewView } from "./types";

export function treeviewInit(view: TreeviewView): TreeviewInitAction {
  return {
    type: ACTION_TREEVIEW_INIT,
    payload: view,
  };
}
