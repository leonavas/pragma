import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { onCleanup, signal } from '../lib/reactive.js';

define('m-clock', () => {
  const now = signal(new Date());
  const timer = setInterval(() => now.set(new Date()), 1000);
  onCleanup(() => clearInterval(timer));
  return h('time', { class: 'clock', 'aria-label': 'Hora atual' }, () => now().toLocaleTimeString('pt-BR', { hour12: false }));
});
