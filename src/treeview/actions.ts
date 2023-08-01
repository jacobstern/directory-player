import {
  ACTION_TREEVIEW_INIT,
  ACTION_TREEVIEW_ITEM_UPDATE,
  TreeviewInitAction,
  TreeviewItemUpdateAction,
} from "./actionTypes";
import { TreeviewItem, TreeviewView } from "./types";

export function treeviewInit(view: TreeviewView): TreeviewInitAction {
  return {
    type: ACTION_TREEVIEW_INIT,
    payload: view,
  };
}

export function treeviewItemUpdate(
  item: TreeviewItem,
): TreeviewItemUpdateAction {
  return {
    type: ACTION_TREEVIEW_ITEM_UPDATE,
    payload: item,
  };
}
