import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useDispatch, useSelector, useStore } from "react-redux";

import "./TreeviewListing.css";
import { selectFlatListing } from "./selectors";
import TreeviewRow from "./TreeviewRow";
import { treeviewItemUpdate } from "./actions";
import { TreeviewItem } from "./types";
import { invoke } from "@tauri-apps/api";
import { throttle } from "../utils/throttle";
import { getQueueAtCursor } from "./helpers";
import { AppAction, AppState } from "../types";

const LIST_ITEM_HEIGHT = 26;
const SCROLL_BUFFER = 300;
const SCROLL_THROTTLE_MS = 25;
const RESIZE_THROTTLE_MS = 100;

export default function TreeviewListing() {
  const { getState } = useStore<AppState, AppAction>(); // For playback start action
  const dispatch = useDispatch();
  const flatListing = useSelector(selectFlatListing);

  // TODO: Move to TreeviewRow
  const handleExpandDirectory = useCallback(
    async (path: string) => {
      const result = await invoke<TreeviewItem>("treeview_expand_directory", {
        directoryPath: path,
      });
      dispatch(treeviewItemUpdate(result));
    },
    [dispatch],
  );
  const handleCollapseDirectory = useCallback(
    async (path: string) => {
      const result = await invoke<TreeviewItem>("treeview_collapse_directory", {
        directoryPath: path,
      });
      dispatch(treeviewItemUpdate(result));
    },
    [dispatch],
  );
  const handlePlayback = useCallback(
    async (path: string) => {
      const state = getState();
      const {
        treeview: { items: normalizedItems },
      } = state;
      const queue = getQueueAtCursor(
        path,
        selectFlatListing(state),
        normalizedItems,
      );
      if (queue !== null) {
        const { filePaths: paths, startIndex } = queue;
        await invoke("player_start_playback", {
          filePaths: paths,
          startIndex,
        });
      }
    },
    [getState],
  );

  // TODO: Virtualization in a different component
  // Also, memoize list rendering

  const [scrollContainerHeight, setScrollContainerHeight] = useState(0);
  const [scrollY, setScrollY] = useState(0);
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  useLayoutEffect(() => {
    if (scrollContainerRef.current) {
      const clientRect = scrollContainerRef.current.getBoundingClientRect();
      setScrollContainerHeight(Math.round(clientRect.height));
    }
  }, []);
  const throttledHandleResize = useMemo(
    () =>
      throttle(() => {
        if (scrollContainerRef.current) {
          const clientRect = scrollContainerRef.current.getBoundingClientRect();
          setScrollContainerHeight(Math.round(clientRect.height));
        }
      }, RESIZE_THROTTLE_MS),
    [],
  );
  const throttledHandleScroll = useMemo(
    () =>
      throttle(() => {
        if (scrollContainerRef.current) {
          setScrollY(scrollContainerRef.current.scrollTop);
        }
      }, SCROLL_THROTTLE_MS),
    [],
  );
  useEffect(() => {
    window.addEventListener("resize", throttledHandleResize);
    return () => {
      window.removeEventListener("resize", throttledHandleResize);
    };
  }, [throttledHandleResize]);
  useEffect(() => {
    const refValue = scrollContainerRef.current;
    refValue?.addEventListener("scroll", throttledHandleScroll);
    return () => {
      refValue?.removeEventListener("scroll", throttledHandleScroll);
    };
  }, [throttledHandleScroll]);

  const startIndex = Math.max(
    0,
    Math.floor((scrollY - SCROLL_BUFFER) / LIST_ITEM_HEIGHT),
  );
  const endIndex = Math.min(
    flatListing.length,
    Math.ceil(
      (scrollContainerHeight + scrollY + SCROLL_BUFFER) / LIST_ITEM_HEIGHT,
    ),
  );

  return (
    <div className="treeview-listing" ref={scrollContainerRef}>
      {scrollContainerHeight ? (
        <div style={{ height: LIST_ITEM_HEIGHT * flatListing.length }}>
          <ol
            className="treeview-listing__container"
            style={{
              transform: `translateY(${startIndex * LIST_ITEM_HEIGHT}px)`,
            }}
          >
            {flatListing.map(({ path, depth }, i) =>
              i >= startIndex && i < endIndex ? (
                <TreeviewRow
                  key={path}
                  path={path}
                  depth={depth}
                  onExpandDirectory={handleExpandDirectory}
                  onCollapseDirectory={handleCollapseDirectory}
                  onPlayback={handlePlayback}
                />
              ) : null,
            )}
          </ol>
        </div>
      ) : null}
    </div>
  );
}
