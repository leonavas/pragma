import { define } from '../lib/component.js';
import { h, list } from '../lib/dom.js';
import { brl } from '../format.js';
import { cart, cartTotal, changeQuantity, clearCart, removeFromCart } from '../state/cart.js';

function cartRow(item, id) {
  return h(
    'li',
    { class: 'cart-item' },
    h('span', { class: 'cart-item-name' }, () => item().nome),
    h(
      'span',
      { class: 'cart-item-quantity' },
      h('button', { type: 'button', 'aria-label': () => `Diminuir ${item().nome}`, disabled: () => item().qtd <= 1, onclick: () => changeQuantity(id, -1) }, '−'),
      h('output', { 'aria-label': 'Quantidade' }, () => item().qtd),
      h('button', { type: 'button', 'aria-label': () => `Aumentar ${item().nome}`, disabled: () => item().qtd >= item().estoque, onclick: () => changeQuantity(id, 1) }, '+'),
    ),
    h('span', { class: 'cart-item-subtotal' }, () => brl(item().qtd * item().preco)),
    h('button', { type: 'button', class: 'button button-quiet', 'aria-label': () => `Remover ${item().nome}`, onclick: () => removeFromCart(id) }, 'Remover'),
  );
}

define('m-cart-summary', () => {
  const isEmpty = () => cart().length === 0;
  return h(
    'section',
    { class: 'cart-summary', 'aria-labelledby': 'carrinho-titulo' },
    h('h2', { id: 'carrinho-titulo' }, 'Carrinho'),
    list(h('ul', { class: 'cart-list' }), cart, (item) => item.id, cartRow),
    h('p', { class: 'empty', hidden: () => !isEmpty() }, 'Carrinho vazio'),
    h('p', { class: 'cart-total' }, 'Total: ', h('strong', null, () => brl(cartTotal()))),
    h('button', { type: 'button', class: 'button', disabled: isEmpty, onclick: clearCart }, 'Esvaziar'),
  );
});
