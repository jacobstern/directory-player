import { z } from "zod";
import syncStorage from "../../sync-storage";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/dialog";

export type CurrentDirectory = string | null;

export type UnsubscribeFunction = VoidFunction;

export type ChangeHandler = (newValue: CurrentDirectory) => void;

const STORAGE_KEY = "treeviewDirectory";

export interface TreeviewDirectory {
  /*
   * Initialize and start listening to global events
   */
  init(): Promise<void>;
  onChange(changeHandler: ChangeHandler): UnsubscribeFunction;
  getCurrent(): CurrentDirectory;
}

interface ChangeHandlerItem {
  changeHandler: ChangeHandler;
}

class TreeviewDirectoryImpl implements TreeviewDirectory {
  currentDirectory: CurrentDirectory = null;
  changeHandlers: ChangeHandlerItem[] = [];

  async init() {
    this.currentDirectory = syncStorage.getWithSchema(STORAGE_KEY, z.string());
    await listen("app://menu-event", (event) => {
      const menuItemId = z.string().parse(event.payload);
      this.handleMenuEvent(menuItemId);
    });
  }

  handleMenuEvent(menuItemId: string) {
    if (menuItemId === "open") {
      this.openDialog();
    }
  }

  async openDialog() {
    const selected = await open({ directory: true });
    if (typeof selected === "string") {
      this.currentDirectory = selected;
      this.notifyChangeHandlers();
    }
  }

  getCurrent() {
    return this.currentDirectory;
  }

  onChange(changeHandler: ChangeHandler): VoidFunction {
    const item: ChangeHandlerItem = { changeHandler };
    this.changeHandlers.push(item);
    return () => {
      const index = this.changeHandlers.indexOf(item);
      if (index >= 0) {
        this.changeHandlers.splice(index, 1);
      }
    };
  }

  notifyChangeHandlers() {
    for (const { changeHandler } of this.changeHandlers) {
      changeHandler(this.currentDirectory);
    }
  }
}

const treeviewDirectory = new TreeviewDirectoryImpl();

export default treeviewDirectory;
