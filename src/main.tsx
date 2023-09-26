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
import { initFileListing } from "./treeview/core/file-listing";
import { FileListingContext } from "./treeview/context/file-listing-context";

async function main() {
  await attachConsole();

  const treeviewView = await invoke<TreeviewView>("treeview_get_view");
  const fileListing = await initFileListing();

  const store = createStore(rootReducer);
  store.dispatch(treeviewInit(treeviewView));

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <PlaybackFileProvider>
        <PlaybackStateProvider>
          <FileListingContext.Provider value={fileListing}>
            <Provider store={store}>
              <App />
            </Provider>
          </FileListingContext.Provider>
        </PlaybackStateProvider>
      </PlaybackFileProvider>
    </React.StrictMode>,
  );
  invoke("show_main_window");
}

main();
