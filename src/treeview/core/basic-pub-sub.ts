export type Listener<A extends Array<unknown>> = (...args: A) => void;

interface ListenerItem<A extends Array<unknown>> {
  listener: Listener<A>;
}

export type UnsubscribeFunction = VoidFunction;

export class BasicPubSub<A extends Array<unknown> = []> {
  private _listeners: ListenerItem<A>[] = [];

  listen(listener: Listener<A>): UnsubscribeFunction {
    const item: ListenerItem<A> = { listener };
    this._listeners.push(item);
    return () => {
      const index = this._listeners.indexOf(item);
      if (index >= 0) {
        this._listeners.splice(index, 1);
      }
    };
  }

  notify(...args: A) {
    for (const { listener } of this._listeners) {
      listener(...args);
    }
  }
}
