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
  readRootDir(path: string): Promise<void>;
  getRoot(): Root;
  addChangeListener(listener: ChangeListener): UnsubscribeFunction;
  expandDirectory(path: string): void;
  collapseDirectory(path: string): void;
}

class FileListingImpl implements FileListing {
  private root: Root = null;
  private pubSub = new BasicPubSub();
  private directoryReverseLookup = new Map<string, File>();

  async readRootDir(path: string): Promise<void> {
    const normalizedPath = await normalize(path);
    const listing = await readDir(normalizedPath, { recursive: false });
    this.directoryReverseLookup.clear();
    for (const child of listing) {
      if (child.children !== undefined) {
        this.directoryReverseLookup.set(child.path, child);
      }
    }
    this.root = {
      path: normalizedPath,
      children: listing,
    };
    this.pubSub.notify();
  }

  getRoot() {
    return this.root;
  }

  addChangeListener(listener: ChangeListener): UnsubscribeFunction {
    return this.pubSub.listen(listener);
  }

  async expandDirectory(path: string): Promise<void> {
    const directory = this.directoryReverseLookup.get(path);
    if (directory === undefined) {
      throw new Error(`Failed to locate directory at "${path}"`);
    }
    if (directory.isExpanded) return;
    directory.isExpanded = true;
    const children = await readDir(path, { recursive: false });
    directory.children = children;
    for (const child of children) {
      if (child.children !== undefined) {
        this.directoryReverseLookup.set(child.path, child);
      }
    }
    this.pubSub.notify();
  }

  async collapseDirectory(path: string): Promise<void> {
    const directory = this.directoryReverseLookup.get(path);
    if (directory === undefined) {
      throw new Error(`Failed to locate directory at "${path}"`);
    }
    if (!directory.isExpanded || !directory.children) return;
    for (const child of directory.children) {
      this.directoryReverseLookup.delete(child.path);
    }
    directory.children = [];
    directory.isExpanded = false;
    this.pubSub.notify();
  }
}

export async function initFileListing(): Promise<FileListing> {
  let isDialogOpen = false;

  const handleMenuEvent = (menuItemId: string) => {
    if (menuItemId === "open") {
      openDialog();
    }
  };
  // TODO: Move initialization and menu handling logic out of this file
  await listen("app://menu-event", (event) => {
    const menuItemId = z.string().parse(event.payload);
    handleMenuEvent(menuItemId);
  });
  const fileListing = new FileListingImpl();

  const openDialog = async (): Promise<void> => {
    if (isDialogOpen) {
      return;
    }
    isDialogOpen = true;
    let path: string | string[] | null = null;
    try {
      path = await open({ directory: true });
    } finally {
      isDialogOpen = false;
    }

    if (typeof path === "string") {
      let success = false;
      try {
        await fileListing.readRootDir(path);
        success = true;
      } catch {
        // TODO: Surface error
      }

      if (success) {
        syncStorage.set(DIRECTORY_STORAGE_KEY, path);
      }
    }
  };
  const persistedDir = syncStorage.getWithSchema(
    DIRECTORY_STORAGE_KEY,
    z.string(),
  );
  if (persistedDir !== null) {
    try {
      await fileListing.readRootDir(persistedDir);
    } catch (e) {
      warn(`Failed to load persisted root with "${e}"`);
      syncStorage.set(DIRECTORY_STORAGE_KEY, null);
    }
  }

  return fileListing;
}
