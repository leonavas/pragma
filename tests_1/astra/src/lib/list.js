// Mantém a identidade dos nós; só move quando a ordem realmente muda.
export function keyedList(root, mount, tag = 'div') {
  const entries = new Map();
  return {
    update(items) {
      const keys = new Set(items.map(item => item.id));
      for (const [id, entry] of entries) {
        if (!keys.has(id)) {
          entry.instance.destroy();
          entry.node.remove();
          entries.delete(id);
        }
      }
      let cursor = root.firstElementChild;
      for (const item of items) {
        let entry = entries.get(item.id);
        if (!entry) {
          const node = document.createElement(tag);
          node.dataset.key = item.id;
          entry = { node, instance: mount(node, item) };
          entries.set(item.id, entry);
        } else entry.instance.update(item);
        if (entry.node !== cursor) root.insertBefore(entry.node, cursor);
        cursor = entry.node.nextElementSibling;
      }
    },
    destroy() {
      for (const entry of entries.values()) entry.instance.destroy();
      entries.clear();
      root.replaceChildren();
    },
  };
}
