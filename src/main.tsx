import "./styles/reset.css";
import "./styles/vars.css";
import "./styles.css";

import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api";
import { legacy_createStore as createStore } from "redux";
import { Provider } from "react-redux";
import { attachConsole } from "tauri-plugin-log-api";

import App from "./App";
import { rootReducer } from "./rootReducer";
import { treeviewInit } from "./treeview/actions";
import { TreeviewView } from "./treeview/types";
import { PlaybackFileProvider, PlaybackStateProvider } from "./player";

async function main() {
  await attachConsole();

  const treeviewView = await invoke<TreeviewView>("treeview_get_view");

  const store = createStore(rootReducer);
  store.dispatch(treeviewInit(treeviewView));

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <PlaybackFileProvider>
        <PlaybackStateProvider>
          <Provider store={store}>
            <App />
          </Provider>
        </PlaybackStateProvider>
      </PlaybackFileProvider>
    </React.StrictMode>,
  );
  invoke("show_main_window");
}

main();
