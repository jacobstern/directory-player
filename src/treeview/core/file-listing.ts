import { z } from "zod";
import { open } from "@tauri-apps/api/dialog";
import { listen } from "@tauri-apps/api/event";
import syncStorage from "../../sync-storage";
import { readDir } from "@tauri-apps/api/fs";
import { error, info, warn } from "tauri-plugin-log-api";
import { normalize } from "@tauri-apps/api/path";
import { BasicPubSub } from "./basic-pub-sub";
import { DebouncedEvent, watch } from "tauri-plugin-fs-watch-api";
import { getLastSegment } from "../../utils/path";

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

export interface ListingChangeInfo {
  reset: boolean;
}

export type Root = File | null;

export type ChangeListener = (info: ListingChangeInfo) => void;

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
  private pubSub = new BasicPubSub<[ListingChangeInfo]>();
  private directoryReverseLookup = new Map<string, File>();
  private unwatchFiles: VoidFunction | null = null;

  async readRootDir(path: string): Promise<void> {
    const normalizedPath = await normalize(path);
    if (this.root?.path === normalizedPath) {
      return Promise.resolve();
    }
    const listing = await readDir(normalizedPath, { recursive: false });
    this.unwatchFiles?.();
    this.unwatchFiles = await watch(
      normalizedPath,
      (event) => {
        // Type is defined incorrectly in the library
        this.handleNotify(event as unknown as DebouncedEvent[]);
      },
      { recursive: true },
    );
    this.root = {
      path: normalizedPath,
      children: listing,
      isExpanded: true,
    };
    this.directoryReverseLookup.clear();
    this.directoryReverseLookup.set(normalizedPath, this.root);
    for (const child of listing) {
      if (child.children !== undefined) {
        this.directoryReverseLookup.set(child.path, child);
      }
    }
    this.pubSub.notify({ reset: true });
  }

  private handleNotify(event: DebouncedEvent[]): void {
    if (event.every((e) => getLastSegment(e.path).startsWith("."))) {
      return;
    }
    info(`Refreshing listing for paths ${event.map((e) => e.path).join()}`);
    this.refreshRoot();
  }

  private async refreshRoot(): Promise<void> {
    const originalPath = this.root?.path;
    if (!originalPath) return Promise.resolve();
    const expandedDirectories = [];
    for (const [path, directory] of this.directoryReverseLookup.entries()) {
      if (directory.isExpanded) {
        expandedDirectories.push(path);
      }
    }
    for (const directory of expandedDirectories) {
      const dirty = await this.inPlaceUpdateDirectory(directory);
      if (this.root?.path !== originalPath) {
        break;
      }
      if (dirty) {
        this.pubSub.notify({ reset: false });
      }
    }
  }

  private async inPlaceUpdateDirectory(path: string): Promise<boolean> {
    const current = this.directoryReverseLookup.get(path);
    if (!current) {
      error(`Tried to update ${path} but it was not in the registry`);
      return false;
    }
    if (!current.children) return false;
    const originalPath = this.root?.path;
    const listing = await readDir(path, { recursive: false });
    if (this.root?.path !== originalPath) {
      return false;
    }
    const removeIndices: number[] = [];
    const newChildrenMap = new Map(listing.map((f) => [f.path, f]));
    let dirty = false;
    for (const [i, file] of current.children.entries()) {
      if (newChildrenMap.has(file.path)) {
        const updated = newChildrenMap.get(file.path)!;
        if (updated.name !== file.name) {
          file.name = updated.name;
          dirty = true;
        }
        newChildrenMap.delete(file.path);
      } else {
        removeIndices.push(i);
        this.recursivelyUnregisterDirectories(file.path, true);
        dirty = true;
      }
    }
    while (removeIndices.length) {
      current.children.splice(removeIndices.pop()!, 1);
    }
    // Remaining entries were newly added to the directory
    for (const child of newChildrenMap.values()) {
      if (child.children !== undefined) {
        this.directoryReverseLookup.set(child.path, child);
      }
      current.children.push(child);
      dirty = true;
    }
    return dirty;
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
    await this.inPlaceUpdateDirectory(path);
    directory.isExpanded = true;
    this.pubSub.notify({ reset: false });
  }

  async collapseDirectory(path: string): Promise<void> {
    if (path === this.root?.path) {
      throw new Error("Cannot collapse the root directory");
    }
    const directory = this.directoryReverseLookup.get(path);
    if (directory === undefined) {
      throw new Error(`Failed to locate directory at "${path}"`);
    }
    if (!directory.isExpanded || !directory.children) return Promise.resolve();
    this.recursivelyUnregisterDirectories(path, false);
    directory.children = [];
    directory.isExpanded = false;
    this.pubSub.notify({ reset: false });
  }

  private recursivelyUnregisterDirectories(
    rootPath: string,
    includeRoot: boolean,
  ) {
    for (const key of this.directoryReverseLookup.keys()) {
      if (key.startsWith(rootPath) && (includeRoot || key !== rootPath)) {
        this.directoryReverseLookup.delete(key);
      }
    }
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
