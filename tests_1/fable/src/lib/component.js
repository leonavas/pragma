// Componente = custom element em light DOM. `render` roda a cada conexão,
// dentro de um scope próprio que é descartado no disconnectedCallback.
import { append } from './dom.js';
import { scope } from './reactive.js';

/**
 * @param {string} tag nome do custom element
 * @param {(props: any, children: Node[], host: HTMLElement) => any} render
 *   devolve os nós do componente; `children` são os filhos passados de fora.
 */
export function define(tag, render) {
  customElements.define(
    tag,
    class extends HTMLElement {
      #dispose;
      #children;

      connectedCallback() {
        this.#children ??= [...this.childNodes];
        this.#dispose = scope(() => {
          this.replaceChildren();
          append(this, render(this.props ?? {}, this.#children, this));
        });
      }

      disconnectedCallback() {
        this.#dispose();
      }

      // Com moveBefore() o elemento muda de lugar sem desconectar: mantém o estado.
      connectedMoveCallback() {}
    },
  );
}
