// Templates são constantes confiáveis, usados apenas na montagem.
export function component(root, template) {
  root.innerHTML = template;
  const controller = new AbortController();
  const disposers = [];
  return {
    root,
    find: selector => root.querySelector(selector),
    on(target, type, handler) {
      target.addEventListener(type, handler, { signal: controller.signal });
    },
    own(child) { disposers.push(() => child.destroy()); return child; },
    cleanup(fn) { disposers.push(fn); },
    destroy() {
      controller.abort();
      for (const dispose of disposers.reverse()) dispose();
      root.replaceChildren();
    },
  };
}
export function text(node, value) {
  const next = String(value);
  if (node.textContent !== next) node.textContent = next;
}
export function value(node, next) {
  if (node.value !== next) node.value = next;
}
