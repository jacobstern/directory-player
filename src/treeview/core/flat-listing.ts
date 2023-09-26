import { File, Root } from "./file-listing";
import { FileType, getFileType } from "./file-type";

export interface FlatListingItem {
  fileType: FileType;
  isExpanded?: boolean;
  depth: number;
  name: string;
  path: string;
}

export type FlatListing = FlatListingItem[] | null;

interface FlatListingStackItem {
  depth: number;
  file: File;
}

function filterFiles(file: File): boolean {
  if (file.name === undefined) {
    return false;
  }
  if (file.name.startsWith(".")) {
    return false;
  }
  return true;
}

function recursivelyFlatten(files: File[]): FlatListingItem[] {
  const listing: FlatListingItem[] = [];
  const collator = new Intl.Collator();
  const compareFiles = (l: File, r: File) => collator.compare(r.name!, l.name!);
  const rootFiles = files.filter(filterFiles).sort(compareFiles);
  const stack: FlatListingStackItem[] = rootFiles.map((file) => ({
    depth: 0,
    file,
  }));
  while (stack.length > 0) {
    const { file, depth } = stack.pop()!;
    const { children, name, path, isExpanded } = file;
    listing.push({
      fileType: getFileType(file),
      isExpanded,
      depth,
      name: name!,
      path,
    });
    if (isExpanded && children) {
      for (const child of children.filter(filterFiles).sort(compareFiles)) {
        stack.push({
          depth: depth + 1,
          file: child,
        });
      }
    }
  }
  return listing;
}

export interface Queue {
  filePaths: string[];
  startIndex: number;
}

export function getQueueAtCursor(
  flatListing: FlatListing,
  startPath: string,
): Queue | null {
  if (flatListing === null) {
    return null;
  }

  const startIndex = flatListing.findIndex((item) => item.path === startPath);
  if (startIndex !== -1 && startIndex < flatListing.length) {
    const { depth: startDepth } = flatListing[startIndex]!;
    const paths = [];

    for (let i = startIndex - 1; i >= 0; i--) {
      const { depth, path, fileType } = flatListing[i]!;
      if (depth < startDepth) {
        break;
      }
      if (depth > startDepth) {
        continue;
      }
      if (fileType === "music-file") {
        paths.push(path);
      }
    }

    paths.reverse();
    const queueStartIndex = paths.length;

    for (let i = startIndex; i < flatListing.length; i++) {
      const { depth, path, fileType } = flatListing[i]!;
      if (depth < startDepth) {
        break;
      }
      if (depth > startDepth) {
        continue;
      }
      if (fileType === "music-file") {
        paths.push(path);
      }
    }

    return { filePaths: paths, startIndex: queueStartIndex };
  }
  return null;
}

export function generateFullListing(root: Root): FlatListing {
  if (root) {
    return recursivelyFlatten(root.children);
  }
  return null;
}
