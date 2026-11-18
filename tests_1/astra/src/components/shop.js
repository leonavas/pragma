import { component, text } from '../lib/component.js';
import { keyedList } from '../lib/list.js';
import { Counter } from './common.js';
import { dispatch, cartTotal, money, loadProducts, normalize } from '../state.js';
const categories = { mercearia: 'Mercearia', limpeza: 'Limpeza', hortifruti: 'Hortifruti' };
export const ProductCard = {
  mount(root, props) {
    const c = component(root, '<article class="product-card"><p class="category"></p><h2><a></a></h2><p class="price"></p><p class="stock"></p><div data-counter></div><button type="button" class="add">Adicionar</button><p class="added" aria-live="polite"></p></article>');
    let qty = 1;
    const counterProps = p => ({ label: 'Qtd', initial: 1, step: 1, min: 1, max: p.estoque, disabled: !p.estoque, onChange: n => { qty = n; } });
    const counter = c.own(Counter.mount(c.find('[data-counter]'), counterProps(props)));
    c.on(c.find('.add'), 'click', () => {
      dispatch({ type: 'cart-add', id: props.id, qty });
      text(c.find('.added'), 'Carrinho atualizado.');
    });
    function update(p) {
      props = p;
      text(c.find('.category'), categories[p.categoria]);
      text(c.find('h2 a'), p.nome);
      c.find('h2 a').href = `/loja/${encodeURIComponent(p.id)}`;
      text(c.find('.price'), money(p.preco));
      text(c.find('.stock'), p.estoque ? `${p.estoque} em estoque` : 'Esgotado');
      c.find('.stock').classList.toggle('sold-out', !p.estoque);
      c.find('.add').disabled = !p.estoque;
      counter.update(counterProps(p));
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const ProductGrid = {
  mount(root, props) {
    const c = component(root, '<div data-loading role="status" aria-label="Carregando produtos"><p class="muted">Carregando produtos…</p><div class="product-grid skeletons"><div></div><div></div><div></div></div></div><div class="empty" data-error hidden><h2>Não foi possível carregar os produtos.</h2><p>Tente novamente para abrir o catálogo.</p><button type="button">Tentar de novo</button></div><p class="empty" data-empty hidden></p><div class="product-grid" data-products></div>');
    const list = c.own(keyedList(c.find('[data-products]'), ProductCard.mount));
    let forcedLoading = false;
    let forced = false;
    let timer;
    c.cleanup(() => clearTimeout(timer));
    c.on(c.find('[data-error] button'), 'click', () => props.onRetry());
    function update(p) {
      const firstForced = p.force && !forced;
      props = p;
      forced = p.force;
      if (firstForced) {
        forcedLoading = true;
        clearTimeout(timer);
        timer = setTimeout(() => { forcedLoading = false; update(props); }, 400);
      }
      const status = forced ? (forcedLoading ? 'loading' : 'error') : p.state.products.status;
      const loading = status === 'idle' || status === 'loading';
      const error = status === 'error';
      const items = loading || error ? [] : p.state.products.data.filter(product => p.id ? product.id === p.id :
        (!p.category || product.categoria === p.category) && normalize(product.nome).includes(normalize(p.search)));
      c.find('[data-loading]').hidden = !loading;
      c.find('[data-error]').hidden = !error;
      c.find('[data-empty]').hidden = loading || error || items.length > 0;
      text(c.find('[data-empty]'), p.id ? 'Produto não encontrado' : 'Nenhum produto');
      list.update(items);
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
const CartItem = {
  mount(root, props) {
    const c = component(root, '<div class="cart-item"><h3></h3><p class="muted subtotal"></p><div class="cart-item-actions"><div class="stepper"><button type="button" data-minus>−</button><output></output><button type="button" data-plus>+</button></div><button type="button" class="quiet" data-remove>Remover</button></div></div>');
    c.on(c.find('[data-minus]'), 'click', () => dispatch({ type: 'cart-quantity', id: props.id, delta: -1 }));
    c.on(c.find('[data-plus]'), 'click', () => dispatch({ type: 'cart-quantity', id: props.id, delta: 1 }));
    c.on(c.find('[data-remove]'), 'click', () => dispatch({ type: 'cart-remove', id: props.id }));
    function update(p) {
      props = p;
      text(c.find('h3'), p.nome);
      text(c.find('.subtotal'), money(Math.round(p.preco * 100) * p.qty / 100));
      text(c.find('output'), p.qty);
      c.find('[data-minus]').disabled = p.qty <= 1;
      c.find('[data-plus]').disabled = p.qty >= p.estoque;
      c.find('[data-minus]').setAttribute('aria-label', `Diminuir ${p.nome}`);
      c.find('[data-plus]').setAttribute('aria-label', `Aumentar ${p.nome}`);
      c.find('[data-remove]').setAttribute('aria-label', `Remover ${p.nome}`);
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const CartSummary = {
  mount(root, props) {
    const c = component(root, '<aside class="cart-summary" aria-label="Seu carrinho"><h2>Seu carrinho</h2><p class="muted" data-empty>O carrinho está vazio.</p><ul></ul><div class="cart-total"><span>Total</span><strong></strong></div><button type="button" class="secondary">Esvaziar</button></aside>');
    const list = c.own(keyedList(c.find('ul'), CartItem.mount, 'li'));
    c.on(c.find('button'), 'click', () => dispatch({ type: 'cart-clear' }));
    function update(p) {
      list.update(p.state.cart);
      c.find('[data-empty]').hidden = p.state.cart.length > 0;
      text(c.find('strong'), money(cartTotal(p.state)));
      c.find('aside > button').disabled = !p.state.cart.length;
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
