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

function fileComparator(l: File, r: File): number {
  const lName = l.name ?? "";
  const rName = r.name ?? "";
  return lName.localeCompare(rName);
}

function recursivelyFlatten(files: File[]): FlatListingItem[] {
  const listing: FlatListingItem[] = [];
  const sortedFiles = files.slice().sort(fileComparator);
  const stack: FlatListingStackItem[] = sortedFiles.map((file) => ({
    depth: 0,
    file,
  }));
  while (stack.length > 0) {
    const { file, depth } = stack.pop()!;
    const { children, name, path, isExpanded } = file;
    if (name === undefined) continue;
    listing.push({
      fileType: getFileType(file),
      isExpanded,
      depth,
      name,
      path,
    });
    if (isExpanded && children) {
      for (const child of children.slice().sort(fileComparator)) {
        stack.push({
          depth: depth + 1,
          file: child,
        });
      }
    }
  }
  return listing;
}

export function generateFullListing(root: Root): FlatListing {
  if (root) {
    return recursivelyFlatten(root.children);
  }
  return null;
}
