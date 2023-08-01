import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useDispatch, useSelector } from "react-redux";

import "./TreeviewListing.css";
import { selectFlatListing } from "./selectors";
import TreeviewRow from "./TreeviewRow";
import { treeviewItemUpdate } from "./actions";
import { TreeviewItem } from "./types";
import { invoke } from "@tauri-apps/api";
import { throttle } from "../utils/throttle";

const LIST_ITEM_HEIGHT = 26;
const SCROLL_BUFFER = 500;
const SCROLL_THROTTLE_MS = 50;
const RESIZE_THROTTLE_MS = 100;

function TreeviewListing() {
  const [scrollContainerHeight, setScrollContainerHeight] = useState<
    number | undefined
  >();
  const [scrollY, setScrollY] = useState(0);
  const dispatch = useDispatch();
  const flatListing = useSelector(selectFlatListing);
  const handleExpandDirectory = useCallback(
    (path: string) => {
      invoke("treeview_expand_directory", { clientPath: path }).then(
        (result) => {
          dispatch(treeviewItemUpdate(result as TreeviewItem));
        },
      );
    },
    [dispatch],
  );
  const handleCollapseDirectory = useCallback(
    (path: string) => {
      invoke("treeview_collapse_directory", { clientPath: path }).then(
        (result) => {
          dispatch(treeviewItemUpdate(result as TreeviewItem));
        },
      );
    },
    [dispatch],
  );
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
      ((scrollContainerHeight ?? 0) + scrollY + SCROLL_BUFFER) /
        LIST_ITEM_HEIGHT,
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
                />
              ) : null,
            )}
          </ol>
        </div>
      ) : null}
    </div>
  );
}

export default memo(TreeviewListing);
