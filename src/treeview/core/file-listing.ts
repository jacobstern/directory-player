import { z } from "zod";
import { open } from "@tauri-apps/api/dialog";
import { listen } from "@tauri-apps/api/event";
import syncStorage from "../../sync-storage";
import { readDir } from "@tauri-apps/api/fs";
import { warn } from "tauri-plugin-log-api";
import { normalize } from "@tauri-apps/api/path";
import { BasicPubSub } from "./basic-pub-sub";

const DIRECTORY_STORAGE_KEY = "treeviewDirectory";

export interface File {
  /**
   * Following the Tauri API, a missing children property indicates
   * that this is not a directory.
   */
  children?: File[];
  /**
   * If this is not set then the children list is not up to date and
   * should be ignored.
   */
  isExpanded?: boolean;
  name?: string;
  path: string;
}

export interface RootDirectory {
  path: string;
  children: File[];
}

export type Root = RootDirectory | null;

export type ChangeListener = () => void;

export type UnsubscribeFunction = VoidFunction;

export interface FileListing {
  getRoot(): Root;
  addChangeListener(listener: ChangeListener): UnsubscribeFunction;
  /**
   * Detach event listeners etc.
   */
  dispose(): void;
}

export async function initFileListing(): Promise<FileListing> {
  let root: Root = null;
  const pubSub = new BasicPubSub();

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
        pubSub.notify();
      }
    }
  };
  const readRootDir = async (path: string): Promise<RootDirectory> => {
    const normalizedPath = await normalize(path);
    const listing = await readDir(normalizedPath, { recursive: false });
    return {
      path: normalizedPath,
      children: listing,
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
    addChangeListener(listener) {
      return pubSub.listen(listener);
    },
    dispose() {
      unlistenMenuEvent();
    },
  };
}
