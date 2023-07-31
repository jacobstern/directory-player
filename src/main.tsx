import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { invoke } from "@tauri-apps/api";

async function main() {
  const res = await invoke("treeview_get_view");
  console.log(res);
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

main();
