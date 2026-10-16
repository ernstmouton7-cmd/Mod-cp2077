/**
 * Synchronous typed event bus. Systems publish gameplay events (horn, crash,
 * violation, signal change...) and renderers / audio / mod adapters subscribe
 * or drain the per-step queue.
 */
export type Listener<T> = (event: T) => void;

export class EventBus<E extends { type: string }> {
  private listeners: Map<string, Listener<E>[]> = new Map();
  private anyListeners: Listener<E>[] = [];
  /** Events emitted since the last drain(); renderers poll this once per frame. */
  private queue: E[] = [];
  private queueLimit = 512;

  on(type: E["type"], fn: Listener<E>): () => void {
    let list = this.listeners.get(type);
    if (list === undefined) {
      list = [];
      this.listeners.set(type, list);
    }
    list.push(fn);
    return () => {
      const l = this.listeners.get(type);
      if (l !== undefined) {
        const i = l.indexOf(fn);
        if (i >= 0) l.splice(i, 1);
      }
    };
  }

  onAny(fn: Listener<E>): void {
    this.anyListeners.push(fn);
  }

  emit(event: E): void {
    const list = this.listeners.get(event.type);
    if (list !== undefined) {
      for (let i = 0; i < list.length; i++) list[i](event);
    }
    for (let i = 0; i < this.anyListeners.length; i++) this.anyListeners[i](event);
    if (this.queue.length < this.queueLimit) this.queue.push(event);
  }

  /** Returns and clears queued events. */
  drain(): E[] {
    const q = this.queue;
    this.queue = [];
    return q;
  }
}
