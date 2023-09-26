import { union, z } from "zod";
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
  expandDirectory(path: string): void;
  collapseDirectory(path: string): void;
  /**
   * Detach event listeners etc.
   */
  dispose(): void;
}

export async function initFileListing(): Promise<FileListing> {
  let root: Root = null;
  const pubSub = new BasicPubSub();
  const directoryReverseLookup = new Map<string, File>();

  const handleMenuEvent = (menuItemId: string) => {
    if (menuItemId === "open") {
      openDialog();
    }
  };
  const openDialog = async (): Promise<void> => {
    const path = await open({ directory: true });
    if (typeof path === "string") {
      let success = false;
      try {
        await readRootDir(path);
        success = true;
      } catch {
        // TODO: Surface error
      }

      if (success) {
        syncStorage.set(DIRECTORY_STORAGE_KEY, path);
        pubSub.notify();
      }
    }
  };
  const readRootDir = async (path: string): Promise<void> => {
    const normalizedPath = await normalize(path);
    const listing = await readDir(normalizedPath, { recursive: false });
    directoryReverseLookup.clear();
    for (const child of listing) {
      if (child.children !== undefined) {
        directoryReverseLookup.set(child.path, child);
      }
    }
    root = {
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
      await readRootDir(persistedDir);
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
    async expandDirectory(path) {
      const directory = directoryReverseLookup.get(path);
      if (directory === undefined) {
        throw new Error(`Failed to locate directory at "${path}"`);
      }
      if (directory.isExpanded) return;
      directory.isExpanded = true;
      const children = await readDir(path, { recursive: false });
      directory.children = children;
      for (const child of children) {
        if (child.children !== undefined) {
          directoryReverseLookup.set(child.path, child);
        }
      }
      pubSub.notify();
    },
    async collapseDirectory(path) {
      const directory = directoryReverseLookup.get(path);
      if (directory === undefined) {
        throw new Error(`Failed to locate directory at "${path}"`);
      }
      if (!directory.isExpanded || !directory.children) return;
      for (const child of directory.children) {
        directoryReverseLookup.delete(child.path);
      }
      directory.children = [];
      pubSub.notify();
    },
    dispose() {
      unlistenMenuEvent();
    },
  };
}
