import "./treeview-listing.styles.css";
import { FlatListingItem, getQueueAtCursor } from "../../../core/flat-listing";
import RowListItem from "./row-list-item";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api";
import { usePlaybackFile } from "../../../../player";
import { throttle } from "../../../../utils/throttle";

const LIST_ITEM_HEIGHT = 26;
/**
 * Rows rendered above and below the virtualization window.
 */
const N_BUFFER_ROWS = 10;
const RESIZE_THROTTLE_MILLIS = 150;

export interface TreeviewListingProps {
  onExpandDirectory: (path: string) => void;
  onCollapseDirectory: (path: string) => void;
  flatListing: FlatListingItem[];
}

export default function TreeviewListing({
  flatListing,
  onExpandDirectory,
  onCollapseDirectory,
}: TreeviewListingProps) {
  const handlePlayback = useCallback(
    async (path: string) => {
      const queue = getQueueAtCursor(flatListing, path);
      if (queue !== null) {
        const { filePaths, startIndex } = queue;
        await invoke("player_start_playback", { filePaths, startIndex });
      }
    },
    [flatListing],
  );
  const playbackFile = usePlaybackFile();

  const [scrollContainerHeight, setScrollContainerHeight] = useState(0);
  const [scrollY, setScrollY] = useState(0);
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  useLayoutEffect(() => {
    const clientRect = scrollContainerRef.current!.getBoundingClientRect();
    setScrollContainerHeight(Math.round(clientRect.height));
  }, []);
  const handleScroll = () => {
    setScrollY(scrollContainerRef.current!.scrollTop);
  };
  useEffect(() => {
    const handleResize = throttle(() => {
      const clientRect = scrollContainerRef.current!.getBoundingClientRect();
      setScrollContainerHeight(Math.round(clientRect.height));
    }, RESIZE_THROTTLE_MILLIS);
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, []);

  const startIndex = Math.max(
    0,
    Math.floor(scrollY / LIST_ITEM_HEIGHT) - N_BUFFER_ROWS,
  );
  const endIndex = Math.min(
    flatListing.length,
    Math.ceil((scrollContainerHeight + scrollY) / LIST_ITEM_HEIGHT) +
      N_BUFFER_ROWS,
  );

  return (
    <div
      className="treeview-listing"
      onScroll={handleScroll}
      ref={scrollContainerRef}
    >
      <div style={{ height: LIST_ITEM_HEIGHT * flatListing.length }}>
        <ol
          className="treeview-listing__container"
          style={{
            transform: `translateY(${startIndex * LIST_ITEM_HEIGHT}px)`,
          }}
        >
          {flatListing.map(
            ({ path, name, fileType, depth, isExpanded }, i) =>
              i >= startIndex &&
              i < endIndex && (
                <RowListItem
                  key={path}
                  path={path}
                  isPlaying={playbackFile?.path === path}
                  name={name}
                  fileType={fileType}
                  depth={depth}
                  isExpanded={isExpanded}
                  onExpandDirectory={onExpandDirectory}
                  onCollapseDirectory={onCollapseDirectory}
                  onPlayback={handlePlayback}
                />
              ),
          )}
        </ol>
      </div>
    </div>
  );
}
