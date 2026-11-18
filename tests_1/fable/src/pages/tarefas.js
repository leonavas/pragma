import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { query } from '../lib/router.js';
import { computed, signal } from '../lib/reactive.js';
import { normalize } from '../format.js';
import { clearDone, doneCount, todos, totalCount } from '../state/todos.js';
import '../components/filter-bar.js';
import '../components/modal.js';
import '../components/search-input.js';
import '../components/stat-card.js';
import '../components/todo-form.js';
import '../components/todo-list.js';

const FILTERS = [
  ['', 'Todas'],
  ['pendentes', 'Pendentes'],
  ['concluidas', 'Concluídas'],
];

define('m-page-tarefas', () => {
  const searchText = signal('');
  const filter = query('filtro');
  const confirming = signal(false);
  const visible = computed(() => {
    const text = normalize(searchText());
    const wantDone = { pendentes: false, concluidas: true }[filter()];
    return todos().filter((todo) => (wantDone === undefined || todo.done === wantDone) && normalize(todo.title).includes(text));
  });
  const closeConfirm = () => confirming.set(false);
  return [
    h('h1', null, 'Tarefas'),
    h(
      'div',
      { class: 'grid' },
      h('m-stat-card', {
        title: 'Concluídas',
        value: () => `${doneCount()} de ${totalCount()}`,
        tone: () => (totalCount() > 0 && doneCount() === totalCount() ? 'ok' : 'neutro'),
      }),
    ),
    h('m-todo-form'),
    h(
      'div',
      { class: 'toolbar' },
      h('m-search-input', { label: 'Buscar tarefa', placeholder: 'Buscar tarefa…', value: searchText, onInput: searchText.set }),
      h('m-filter-bar', { label: 'Filtrar tarefas', param: 'filtro', options: FILTERS }),
    ),
    h('m-todo-list', { items: visible }),
    h('button', { type: 'button', class: 'button', disabled: () => doneCount() === 0, onclick: () => confirming.set(true) }, 'Limpar concluídas'),
    h(
      'm-modal',
      { title: 'Limpar concluídas?', open: confirming, onClose: closeConfirm },
      h('p', null, () => `${doneCount()} ${doneCount() === 1 ? 'tarefa concluída será removida' : 'tarefas concluídas serão removidas'}.`),
      h(
        'div',
        { class: 'row' },
        h('button', { type: 'button', class: 'button', onclick: closeConfirm }, 'Cancelar'),
        h(
          'button',
          {
            type: 'button',
            class: 'button button-primary',
            onclick: () => {
              clearDone();
              closeConfirm();
            },
          },
          'Confirmar',
        ),
      ),
    ),
  ];
});
