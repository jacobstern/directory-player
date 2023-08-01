import { treeviewReducer } from "./treeview/reducers";
import { AppAction, AppState } from "./types";
import { Reducer, combineReducers } from "redux";

export const rootReducer: Reducer<AppState, AppAction> = combineReducers({
  treeview: treeviewReducer,
});
