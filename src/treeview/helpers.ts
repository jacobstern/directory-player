import { FlatListingItem } from "./selectors";

export function getPlaybackItems(
  startPath: string,
  flatListing: FlatListingItem[],
): string[] {
  const startIndex = flatListing.findIndex((item) => item.path === startPath);
  if (startIndex !== -1) {
    const { depth: startDepth } = flatListing[startIndex];
    const paths = [];
    for (let i = startIndex; i < flatListing.length; i++) {
      const { depth, path, canPlay } = flatListing[i];
      if (depth < startDepth) {
        break;
      }
      if (depth > startDepth) {
        continue;
      }
      if (canPlay) {
        paths.push(path);
      }
    }
    return paths;
  }
  return [];
}
