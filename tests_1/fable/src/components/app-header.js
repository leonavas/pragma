import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { path } from '../lib/router.js';
import { cartCount } from '../state/cart.js';

// Props: href. Ativo na própria rota e nas subrotas (/loja/:id marca "Loja").
define('m-nav-link', ({ href }, children) => {
  const isActive = () => path() === href || (href !== '/' && path().startsWith(`${href}/`));
  return h('a', { class: 'nav-link', href, 'aria-current': () => (isActive() ? 'page' : null) }, children);
});

define('m-cart-badge', () =>
  h(
    'a',
    { class: 'cart-badge', href: '/loja', 'aria-label': () => `Carrinho: ${cartCount()} ${cartCount() === 1 ? 'item' : 'itens'}` },
    'Carrinho ',
    h('span', { class: 'cart-badge-count' }, cartCount),
  ),
);

define('m-app-header', () =>
  h(
    'header',
    { class: 'app-header' },
    h('a', { class: 'app-name', href: '/' }, 'Mercadinho'),
    h(
      'nav',
      { class: 'app-nav', 'aria-label': 'Principal' },
      h('m-nav-link', { href: '/' }, 'Painel'),
      h('m-nav-link', { href: '/tarefas' }, 'Tarefas'),
      h('m-nav-link', { href: '/loja' }, 'Loja'),
    ),
    h('m-cart-badge'),
  ),
);
