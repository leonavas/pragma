import { define } from '../lib/component.js';
import { h, list } from '../lib/dom.js';
import { loadProducts, productsStatus, shouldFail } from '../state/products.js';
import './product-card.js';

const skeleton = () =>
  h(
    'ul',
    { class: 'product-grid', 'aria-busy': 'true', 'aria-label': 'Carregando produtos' },
    Array.from({ length: 6 }, () => h('li', { class: 'skeleton' })),
  );

// Props: items (getter dos produtos já filtrados).
define('m-product-grid', ({ items }) => {
  loadProducts({ fail: shouldFail() });
  return () => {
    const status = productsStatus();
    if (status === 'error') {
      return h(
        'div',
        { class: 'notice', role: 'alert' },
        h('p', null, 'Não foi possível carregar os produtos.'),
        h('button', { type: 'button', class: 'button', onclick: () => loadProducts() }, 'Tentar de novo'),
      );
    }
    if (status !== 'ready') return skeleton();
    return [
      list(
        h('ul', { class: 'product-grid' }),
        items,
        (product) => product.id,
        (product) => h('li', null, h('m-product-card', { product: product() })),
      ),
      h('p', { class: 'empty', hidden: () => items().length > 0 }, 'Nenhum produto'),
    ];
  };
});
