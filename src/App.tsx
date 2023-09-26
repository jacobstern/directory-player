import useGlobalPlayPauseKeyHandler from "./hooks/use-global-play-pause-key-event-handler";
import { PlayerPane } from "./player";
import TreeviewPane from "./treeview/ui/treeview-pane";
import { debug } from "tauri-plugin-log-api";

import "./App.css";
import useEventListener from "./tauri/hooks/use-event-listener";

function App() {
  useGlobalPlayPauseKeyHandler();
  useEventListener("app://menu-event", (e) => {
    debug(`${e.payload}`);
  });

  return (
    <main className="app">
      <PlayerPane />
      <TreeviewPane />
    </main>
  );
}

export default App;
