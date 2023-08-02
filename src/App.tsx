import "./App.css";
import { PlayerPane } from "./player";

import TreeviewPane from "./treeview/TreeviewPane";

function App() {
  return (
    <main className="app">
      <PlayerPane />
      <TreeviewPane />
    </main>
  );
}

export default App;
