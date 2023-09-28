import "./styles/reset.css";
import "./styles/vars.css";
import "./styles.css";

import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api";
import { attachConsole } from "tauri-plugin-log-api";

import App from "./App";
import { PlaybackFileProvider, PlaybackStateProvider } from "./player";
import { initFileListing } from "./treeview/core/file-listing";
import { FileListingContext } from "./treeview/context/file-listing-context";

async function main() {
  await attachConsole();
  const fileListing = await initFileListing();

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <PlaybackFileProvider>
        <PlaybackStateProvider>
          <FileListingContext.Provider value={fileListing}>
            <App />
          </FileListingContext.Provider>
        </PlaybackStateProvider>
      </PlaybackFileProvider>
    </React.StrictMode>,
  );
  invoke("show_main_window");
}

main();
