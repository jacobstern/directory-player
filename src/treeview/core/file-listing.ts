import { z } from "zod";
import { open } from "@tauri-apps/api/dialog";
import { listen } from "@tauri-apps/api/event";
import syncStorage from "../../sync-storage";
import { readDir } from "@tauri-apps/api/fs";
import { warn } from "tauri-plugin-log-api";
import { normalize } from "@tauri-apps/api/path";

const DIRECTORY_STORAGE_KEY = "treeviewDirectory";

export interface File {
  children?: File[];
  name?: string;
  path: string;
}

export interface RootDirectory {
  path: string;
  children: File[];
}

export type Root = RootDirectory | null;

export type ChangeHandler = (type: "root" | "subdir", path?: string) => void;

export interface FileListing {
  getRoot(): Root;
  setChangeHandler(handler: ChangeHandler | null): void;
  /**
   * Detach event listeners etc.
   */
  dispose(): void;
}

export async function initFileListing(): Promise<FileListing> {
  let root: Root = null;
  let changeHandler: ChangeHandler | null = null;

  const handleMenuEvent = (menuItemId: string) => {
    if (menuItemId === "open") {
      openDialog();
    }
  };
  const openDialog = async (): Promise<void> => {
    const path = await open({ directory: true });
    if (typeof path === "string") {
      let updated: Root = null;
      try {
        updated = await readRootDir(path);
      } catch {
        // TODO: Surface error
      }

      if (updated) {
        root = updated;
        syncStorage.set(DIRECTORY_STORAGE_KEY, path);
        changeHandler?.("root", path);
      }
    }
  };
  const readRootDir = async (path: string): Promise<RootDirectory> => {
    const normalizedPath = await normalize(path);
    return {
      path: normalizedPath,
      children: await readDir(normalizedPath),
    };
  };

  const persistedDir = syncStorage.getWithSchema(
    DIRECTORY_STORAGE_KEY,
    z.string(),
  );
  if (persistedDir !== null) {
    try {
      root = await readRootDir(persistedDir);
    } catch (e) {
      warn(`Failed to load persisted root with "${e}"`);
      syncStorage.set(DIRECTORY_STORAGE_KEY, null);
    }
  }
  const unlistenMenuEvent = await listen("app://menu-event", (event) => {
    const menuItemId = z.string().parse(event.payload);
    handleMenuEvent(menuItemId);
  });

  return {
    getRoot() {
      return root;
    },
    setChangeHandler(handler) {
      changeHandler = handler;
    },
    dispose() {
      unlistenMenuEvent();
    },
  };
}
