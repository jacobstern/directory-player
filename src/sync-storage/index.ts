import { z } from "zod";

export interface SyncStorage {
  /**
   * Set a value in persistent storage. The value must be JSON
   * serializable. If the value is `null`, the key is removed instead.
   */
  set(key: string, value: unknown): void;
  /**
   * Get a value from the store. Throws the Zod error if the value is
   * present but cannot be deserialized according to the schema.
   */
  getWithSchema<T>(key: string, schema: z.Schema<T>): T | null;
}

const syncStorage: SyncStorage = {
  set(key, value) {
    if (value === null) {
      localStorage.removeItem(key);
    } else {
      localStorage.setItem(key, JSON.stringify(value));
    }
  },
  getWithSchema(key, schema) {
    const found = localStorage.getItem(key);
    if (found === null) return null;
    return schema.parse(JSON.parse(found));
  },
};

export default syncStorage;
