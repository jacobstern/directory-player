import { useId, useMemo } from "react";
import "./ExpandButton.css";

export interface ExpandButtonProps {
  isExpanded?: boolean;
  onToggle: VoidFunction;
}

function ExpandButton({ isExpanded, onToggle }: ExpandButtonProps) {
  const titleId = useId();
  const icon = useMemo(
    () => (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        height="1em"
        viewBox="0 0 320 512"
        style={{ transform: `rotate(${isExpanded ? "90deg" : "0"}` }}
        stroke="currentColor"
        fill="currentColor"
        aria-labelledby={titleId}
      >
        <title id={titleId}>{isExpanded ? "Collapse" : "Expand"}</title>
        {/*! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc.*/}
        <path d="M278.6 233.4c12.5 12.5 12.5 32.8 0 45.3l-160 160c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L210.7 256 73.4 118.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l160 160z" />
      </svg>
    ),
    [isExpanded, titleId],
  );
  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    onToggle();
  };
  return (
    <button className="expand-button" type="button" onClick={handleClick}>
      {icon}
    </button>
  );
}

export default ExpandButton;
