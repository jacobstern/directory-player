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

export function generateFullListing(root: Root): FlatListing {
  if (root) {
    return recursivelyFlatten(root.children);
  }
  return null;
}
