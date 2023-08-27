import "./App.css";
import { PlayerPane } from "./player";
import PlayerDebugOverlay from "./player/ui/player-debug-overlay";

import TreeviewPane from "./treeview/TreeviewPane";

function App() {
  return (
    <main className="app">
      <PlayerDebugOverlay />
      <PlayerPane />
      <TreeviewPane />
    </main>
  );
}

export default App;
