import { sep } from "@tauri-apps/api/path";

export function getParentPath(path: string): string {
  return path.substring(0, path.lastIndexOf(sep));
}

export function getLastSegment(path: string): string {
  const lastSepIndex = path.lastIndexOf(sep);
  if (lastSepIndex === -1) {
    return path;
  }
  return path.substring(lastSepIndex + 1);
}

export function renameLastSegment(path: string, newName: string): string {
  return [getParentPath(path), newName].join(sep);
}

export function containsPathSeparator(path: string): boolean {
  return path.indexOf(sep) !== -1;
}
