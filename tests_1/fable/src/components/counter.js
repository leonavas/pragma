import { define } from '../lib/component.js';
import { h, read } from '../lib/dom.js';
import { computed, signal } from '../lib/reactive.js';

// Props: label, initial, step, min?, max? (min/max aceitam getter), onChange?(valor).
define('m-counter', ({ label, initial = 0, step = 1, min = -Infinity, max = Infinity, onChange }) => {
  const raw = signal(initial);
  const value = computed(() => Math.min(Math.max(raw(), read(min)), read(max)));
  const change = (delta) => {
    raw.set(value() + delta);
    onChange?.(value());
  };
  return h(
    'div',
    { class: 'counter', role: 'group', 'aria-label': label },
    h('span', { class: 'counter-label' }, label),
    h('button', { type: 'button', 'aria-label': `Diminuir ${label}`, disabled: () => value() - step < read(min), onclick: () => change(-step) }, '−'),
    h('output', { class: 'counter-value' }, value),
    h('button', { type: 'button', 'aria-label': `Aumentar ${label}`, disabled: () => value() + step > read(max), onclick: () => change(step) }, '+'),
  );
});
