export type ChangeHandler<T> = (newValue: T) => void;

export type UnsubscribeFunction = VoidFunction;

interface ChangeHandlerItem<T> {
  changeHandler: ChangeHandler<T>;
}

export interface NotifyChangedReadonly<T> {
  /**
   * Get the current value.
   */
  get(): T;
  /**
   * Subscribe to updates. The callback invocation is synchronous and
   * is not queued in the event loop.
   */
  onChange(changeHandler: ChangeHandler<T>): UnsubscribeFunction;
}

export default class NotifyChanged<T> implements NotifyChangedReadonly<T> {
  private _value: T;
  private _changeHandlers: ChangeHandlerItem<T>[] = [];

  constructor(value: T) {
    this._value = value;
  }

  set(value: T): void {
    if (value !== this._value) {
      this._value = value;
      this._notifyChangeHandlers();
    }
  }

  get(): T {
    return this._value;
  }

  onChange(changeHandler: ChangeHandler<T>): UnsubscribeFunction {
    const item: ChangeHandlerItem<T> = { changeHandler };
    this._changeHandlers.push(item);
    return () => {
      const index = this._changeHandlers.indexOf(item);
      if (index >= 0) {
        this._changeHandlers.splice(index, 1);
      }
    };
  }

  private _notifyChangeHandlers() {
    for (const { changeHandler } of this._changeHandlers) {
      changeHandler(this._value);
    }
  }
}
