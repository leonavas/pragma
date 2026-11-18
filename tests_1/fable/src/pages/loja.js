import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { query } from '../lib/router.js';
import { computed, signal } from '../lib/reactive.js';
import { normalize } from '../format.js';
import { loadProducts, products, productsStatus, shouldFail } from '../state/products.js';
import '../components/cart-summary.js';
import '../components/filter-bar.js';
import '../components/product-card.js';
import '../components/product-grid.js';
import '../components/search-input.js';

const CATEGORIES = [
  ['', 'Todas'],
  ['mercearia', 'Mercearia'],
  ['limpeza', 'Limpeza'],
  ['hortifruti', 'Hortifruti'],
];

define('m-page-loja', () => {
  const searchText = signal('');
  const category = query('categoria');
  const visible = computed(() => {
    const text = normalize(searchText());
    const wanted = CATEGORIES.some(([value]) => value === category()) ? category() : '';
    return products().filter((product) => (!wanted || product.categoria === wanted) && normalize(product.nome).includes(text));
  });
  return [
    h('h1', null, 'Loja'),
    h(
      'div',
      { class: 'toolbar' },
      h('m-search-input', { label: 'Buscar produto', placeholder: 'Buscar produto…', value: searchText, onInput: searchText.set }),
      h('m-filter-bar', { label: 'Filtrar por categoria', param: 'categoria', options: CATEGORIES }),
    ),
    h('div', { class: 'shop' }, h('m-product-grid', { items: visible }), h('m-cart-summary')),
  ];
});

// /loja/:id
define('m-page-produto', ({ params }) => {
  loadProducts({ fail: shouldFail() });
  const product = computed(() => products().find((item) => item.id === params.id));
  return [
    h('p', null, h('a', { href: '/loja' }, '← Voltar para a loja')),
    () => {
      const status = productsStatus();
      if (status === 'error') {
        return h(
          'div',
          { class: 'notice', role: 'alert' },
          h('p', null, 'Não foi possível carregar o produto.'),
          h('button', { type: 'button', class: 'button', onclick: () => loadProducts() }, 'Tentar de novo'),
        );
      }
      if (status !== 'ready') return h('div', { class: 'skeleton product-detail', 'aria-busy': 'true', 'aria-label': 'Carregando produto' });
      if (!product()) return h('p', { class: 'empty' }, 'Produto não encontrado');
      return h('div', { class: 'product-detail' }, h('m-product-card', { product: product(), detail: true }));
    },
  ];
});
