import { sep } from "@tauri-apps/api/path";

export function getParentPath(path: string): string {
  return path.substring(0, path.lastIndexOf(sep));
}
