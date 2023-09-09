import { FlatListingItem } from "./selectors";
import { NormalizedItems } from "./types";

export interface Queue {
  filePaths: string[];
  startIndex: number;
}

export function getQueueAtCursor(
  startPath: string,
  flatListing: FlatListingItem[],
  normalizedItems: NormalizedItems,
): Queue | null {
  const startIndex = flatListing.findIndex((item) => item.path === startPath);
  if (startIndex !== -1 && startIndex < flatListing.length) {
    const { depth: startDepth } = flatListing[startIndex]!;
    const paths = [];

    for (let i = startIndex - 1; i >= 0; i--) {
      const { depth, path } = flatListing[i]!;
      if (depth < startDepth) {
        break;
      }
      if (depth > startDepth) {
        continue;
      }
      const item = normalizedItems[path];
      if (item?.type === "File" && item.canPlay) {
        paths.push(path);
      }
    }

    paths.reverse();
    const queueStartIndex = paths.length;

    for (let i = startIndex; i < flatListing.length; i++) {
      const { depth, path } = flatListing[i]!;
      if (depth < startDepth) {
        break;
      }
      if (depth > startDepth) {
        continue;
      }
      const item = normalizedItems[path];
      if (item?.type === "File" && item.canPlay) {
        paths.push(path);
      }
    }

    return { filePaths: paths, startIndex: queueStartIndex };
  }
  return null;
}
