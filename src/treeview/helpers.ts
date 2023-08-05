import { FlatListingItem } from "./selectors";
import { NormalizedItems, NormalizedTreeviewItem } from "./types";

export function getPlaybackItems(
  startPath: string,
  flatListing: FlatListingItem[],
  normalizedItems: NormalizedItems,
): string[] {
  const startIndex = flatListing.findIndex((item) => item.path === startPath);
  if (startIndex !== -1) {
    const { depth: startDepth } = flatListing[startIndex];
    const paths = [];
    for (let i = startIndex; i < flatListing.length; i++) {
      const { depth, path } = flatListing[i];
      if (depth < startDepth) {
        break;
      }
      if (depth > startDepth) {
        continue;
      }
      const item: NormalizedTreeviewItem | undefined = normalizedItems[path];
      if (item?.type === "File" && item.canPlay) {
        paths.push(path);
      }
    }
    return paths;
  }
  return [];
}
