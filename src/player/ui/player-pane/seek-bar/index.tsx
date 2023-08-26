import { useState } from "react";

import "./seek-bar.styles.css";

export default function SeekBar() {
  const [value, setValue] = useState(0);
  return <progress className="seek-bar" value={value} />;
}
