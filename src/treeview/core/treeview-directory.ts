import { z } from "zod";
import syncStorage from "../../sync-storage";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/dialog";
import NotifyChanged, { NotifyChangedReadonly } from "./notify-changed";
import { warn } from "tauri-plugin-log-api";

export type CurrentDirectory = string | null;

const DIRECTORY_STORAGE_KEY = "treeviewDirectory";

export default class TreeviewDirectory {
  private _currentDirectory = new NotifyChanged<CurrentDirectory>(null);

  get currentDirectory(): NotifyChangedReadonly<CurrentDirectory> {
    return this._currentDirectory;
  }

  /**
   * Initialize and start listening to global events.
   */
  async init() {
    try {
      const persistedValue = syncStorage.getWithSchema(
        DIRECTORY_STORAGE_KEY,
        z.string(),
      );
      this._currentDirectory.set(persistedValue);
    } catch (e) {
      if (e instanceof z.ZodError) {
        warn(`Failed to load current directory with schema error ${e}`);
      } else {
        throw e;
      }
    }
    await listen("app://menu-event", (event) => {
      const menuItemId = z.string().parse(event.payload);
      this._handleMenuEvent(menuItemId);
    });
  }

  private _handleMenuEvent(menuItemId: string) {
    if (menuItemId === "open") {
      this._openDialog();
    }
  }

  private async _openDialog() {
    const selected = await open({ directory: true });
    if (typeof selected === "string") {
      this._currentDirectory.set(selected);
      syncStorage.set(DIRECTORY_STORAGE_KEY, selected);
    }
  }
}
