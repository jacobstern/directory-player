import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { invoke } from "@tauri-apps/api";
import { TreeviewView } from "./treeview/types";
import { legacy_createStore as createStore } from "redux";
import { Provider } from "react-redux";
import { rootReducer } from "./rootReducer";
import { treeviewInit } from "./treeview/actions";
import "./styles.css";

async function main() {
  const treeviewView = await invoke("treeview_get_view");

  const store = createStore(rootReducer);
  store.dispatch(treeviewInit(treeviewView as TreeviewView));
  console.log(store.getState());

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <Provider store={store}>
        <App />
      </Provider>
    </React.StrictMode>,
  );
  invoke("show_main_window");
}

main();
