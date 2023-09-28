import useGlobalPlayPauseKeyHandler from "./hooks/use-global-play-pause-key-event-handler";
import { PlayerPane } from "./player";
import TreeviewPane from "./treeview/ui/treeview-pane";

import "./App.css";

function App() {
  useGlobalPlayPauseKeyHandler();
  return (
    <main className="app">
      <PlayerPane />
      <TreeviewPane />
    </main>
  );
}

export default App;
