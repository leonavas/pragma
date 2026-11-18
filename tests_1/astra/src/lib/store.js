export function createStore(initial, reduce) {
  let state = initial;
  let processing = false;
  const listeners = new Set();
  const queue = [];
  return {
    getState: () => state,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch(action) {
      queue.push(action);
      if (processing) return;
      processing = true;
      try {
        while (queue.length) {
          const next = reduce(state, queue.shift());
          if (next === state) continue;
          state = next;
          for (const listener of listeners) listener(state);
        }
      } finally { processing = false; }
    },
  };
}
