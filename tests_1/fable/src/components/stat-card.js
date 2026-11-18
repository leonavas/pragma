import { define } from '../lib/component.js';
import { h, read } from '../lib/dom.js';

// Props: title, value, hint?, tone ('neutro' | 'ok' | 'alerta'). value, hint e tone aceitam getter.
define('m-stat-card', ({ title, value, hint, tone = 'neutro' }) =>
  h(
    'article',
    { class: 'stat-card', 'data-tone': () => read(tone) },
    h('h3', { class: 'stat-title' }, title),
    h('p', { class: 'stat-value' }, () => read(value)),
    hint && h('p', { class: 'stat-hint' }, () => read(hint)),
  ),
);
