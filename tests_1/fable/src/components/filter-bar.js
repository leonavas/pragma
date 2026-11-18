import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { query, withQuery } from '../lib/router.js';

// O valor vive na URL (?param=valor). Cada opção é um link: voltar/avançar,
// refresh e "abrir em nova aba" funcionam sem código extra.
// Props: label, param, options: [valor, rótulo][] (valor '' = sem filtro).
define('m-filter-bar', ({ label, param, options }) => {
  const current = query(param);
  const isActive = (value) => (current() ?? '') === value || (!value && !options.some(([option]) => option === current()));
  return h(
    'nav',
    { class: 'filter-bar', 'aria-label': label },
    options.map(([value, text]) =>
      h('a', { class: 'filter-option', href: () => withQuery(param, value), 'aria-current': () => (isActive(value) ? 'true' : null) }, text),
    ),
  );
});
