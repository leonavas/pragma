import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { signal } from '../lib/reactive.js';
import { uid } from '../format.js';

// Composição: cada filho elemento é um painel; o rótulo da aba vem de `data-tab`.
// Props: label (nome acessível do conjunto). Painéis inativos ficam ocultos,
// não desmontados: o estado dentro deles é preservado.
define('m-tabs', ({ label }, children) => {
  const panels = children.filter((node) => node.nodeType === 1);
  const selected = signal(0);
  const ids = panels.map(() => uid('tab'));

  const tabs = panels.map((panel, index) =>
    h(
      'button',
      {
        type: 'button',
        role: 'tab',
        id: ids[index],
        'aria-controls': `${ids[index]}-painel`,
        'aria-selected': () => String(selected() === index),
        tabindex: () => (selected() === index ? 0 : -1),
        onclick: () => selected.set(index),
      },
      panel.dataset.tab,
    ),
  );

  const onKeydown = (event) => {
    const last = panels.length - 1;
    const current = selected();
    const next = { ArrowRight: current === last ? 0 : current + 1, ArrowLeft: current === 0 ? last : current - 1, Home: 0, End: last }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    selected.set(next);
    tabs[next].focus();
  };

  return [
    h('div', { class: 'tabs-list', role: 'tablist', 'aria-label': label, onkeydown: onKeydown }, tabs),
    panels.map((panel, index) =>
      h(
        'div',
        { class: 'tabs-panel', role: 'tabpanel', id: `${ids[index]}-painel`, 'aria-labelledby': ids[index], tabindex: 0, hidden: () => selected() !== index },
        panel,
      ),
    ),
  ];
});
