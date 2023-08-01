import "./css/vendor/reset.css";
import "./css/vars.css"; // Before main.css
import "./main.css";

import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api";
import { legacy_createStore as createStore } from "redux";
import { Provider } from "react-redux";

import App from "./App";
import { rootReducer } from "./rootReducer";
import { treeviewInit } from "./treeview/actions";
import { TreeviewView } from "./treeview/types";

async function main() {
  const treeviewView = await invoke("treeview_get_view");

  const store = createStore(rootReducer);
  store.dispatch(treeviewInit(treeviewView as TreeviewView));

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
