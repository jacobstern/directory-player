import { TreeviewAction } from "./treeview/actionTypes";
import { TreeviewState } from "./treeview/types";

/**
 * Stand-in for arbitrary unknown actions. Do not handle this.
 */
interface UnknownAction {
  type: "@@redux/PROBE_UNKNOWN_ACTION_z.r.p.l.z";
  [extraProps: string]: unknown;
}

export type AppAction = TreeviewAction | UnknownAction;

export type AppState = {
  treeview: TreeviewState;
};
