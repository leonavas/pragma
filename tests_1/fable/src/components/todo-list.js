import { define } from '../lib/component.js';
import { h, list } from '../lib/dom.js';
import { batch, signal } from '../lib/reactive.js';
import { removeTodo, renameTodo, toggleTodo } from '../state/todos.js';

// `todo` é o getter da linha: acompanha o objeto atual daquela chave, então a
// linha se atualiza no lugar e o estado de edição sobrevive às mudanças da lista.
function todoRow(todo, id) {
  const editing = signal(false);
  const draft = signal('');
  const error = signal('');

  const stopEditing = () => editing.set(false);
  const startEditing = () => {
    batch(() => {
      draft.set(todo().title);
      error.set('');
      editing.set(true);
    });
    input.focus();
    input.select();
  };
  const save = () => {
    const message = renameTodo(id, draft());
    batch(() => {
      error.set(message);
      if (!message) editing.set(false);
    });
  };

  const input = h('input', {
    class: 'input todo-edit',
    'aria-label': 'Editar título da tarefa',
    value: draft,
    hidden: () => !editing(),
    'aria-invalid': () => (error() ? 'true' : null),
    oninput: (event) => draft.set(event.target.value),
    onblur: stopEditing,
    onkeydown: (event) => {
      if (event.key === 'Enter') save();
      else if (event.key === 'Escape') stopEditing();
    },
  });

  return h(
    'li',
    { class: () => (todo().done ? 'todo-item is-done' : 'todo-item') },
    h('input', {
      type: 'checkbox',
      checked: () => todo().done,
      'aria-label': () => `Concluir: ${todo().title}`,
      onchange: () => toggleTodo(id),
    }),
    h('span', { class: 'todo-title', title: 'Duplo clique para editar', hidden: editing, ondblclick: startEditing }, () => todo().title),
    input,
    h('button', { type: 'button', class: 'button button-quiet', 'aria-label': () => `Remover: ${todo().title}`, onclick: () => removeTodo(id) }, 'Remover'),
    h('p', { class: 'field-error', role: 'alert', hidden: () => !editing() || !error() }, error),
  );
}

// Props: items (getter da lista já filtrada).
define('m-todo-list', ({ items }) => [
  list(h('ul', { class: 'todo-list' }), items, (todo) => todo.id, todoRow),
  h('p', { class: 'empty', hidden: () => items().length > 0 }, 'Nenhuma tarefa'),
]);
