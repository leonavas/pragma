import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { signal } from '../lib/reactive.js';
import { brl } from '../format.js';
import { cartCount, cartTotal } from '../state/cart.js';
import { pendingCount, totalCount } from '../state/todos.js';
import '../components/clock.js';
import '../components/counter.js';
import '../components/modal.js';
import '../components/stat-card.js';
import '../components/tabs.js';
import '../components/theme-toggle.js';

define('m-page-painel', () => {
  const aboutOpen = signal(false);
  const cartTone = () => (cartCount() === 0 ? 'alerta' : 'ok');
  return [
    h('div', { class: 'page-header' }, h('h1', null, 'Painel'), h('m-clock'), h('m-theme-toggle')),
    h(
      'section',
      { class: 'grid', 'aria-label': 'Resumo' },
      h('m-stat-card', {
        title: 'Tarefas pendentes',
        value: pendingCount,
        hint: () => `de ${totalCount()} no total`,
        tone: () => (pendingCount() === 0 ? 'alerta' : 'neutro'),
      }),
      h('m-stat-card', { title: 'Itens no carrinho', value: cartCount, hint: 'somando as quantidades', tone: cartTone }),
      h('m-stat-card', { title: 'Total do carrinho', value: () => brl(cartTotal()), tone: cartTone }),
    ),
    h(
      'section',
      { class: 'panel' },
      h('h2', null, 'Contadores'),
      h(
        'div',
        { class: 'row' },
        h('m-counter', { label: 'Cliques', initial: 0, step: 1 }),
        h('m-counter', { label: 'Pulos de 5', initial: 10, step: 5, min: 0, max: 50 }),
      ),
    ),
    h(
      'section',
      { class: 'panel' },
      h('h2', null, 'Informações'),
      h(
        'm-tabs',
        { label: 'Informações' },
        h('div', { 'data-tab': 'Sobre' }, h('p', null, 'Mercadinho é um app de exemplo: painel, tarefas e uma loja com carrinho.')),
        h(
          'div',
          { 'data-tab': 'Atalhos' },
          h(
            'ul',
            { class: 'bullets' },
            h('li', null, '← e → trocam de aba; Home e End vão à primeira e à última.'),
            h('li', null, 'Esc fecha janelas.'),
            h('li', null, 'Duplo clique em uma tarefa edita o título; Enter salva e Esc cancela.'),
          ),
        ),
        h('div', { 'data-tab': 'Contador' }, h('m-counter', { label: 'Na aba', initial: 3, step: 3 })),
      ),
    ),
    h('button', { type: 'button', class: 'button', onclick: () => aboutOpen.set(true) }, 'Sobre o app'),
    h(
      'm-modal',
      { title: 'Sobre o app', open: aboutOpen, onClose: () => aboutOpen.set(false) },
      h('p', null, 'Feito só com HTML, CSS e JS nativos: signals próprios e custom elements, sem framework.'),
    ),
  ];
});
