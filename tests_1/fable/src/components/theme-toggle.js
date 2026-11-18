import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { theme, toggleTheme } from '../state/theme.js';

define('m-theme-toggle', () =>
  h(
    'button',
    { type: 'button', class: 'button', 'aria-pressed': () => String(theme() === 'dark'), onclick: toggleTheme },
    () => (theme() === 'dark' ? 'Tema escuro: ligado' : 'Tema escuro: desligado'),
  ),
);
