import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { batch, signal } from '../lib/reactive.js';
import { uid } from '../format.js';
import { addTodo } from '../state/todos.js';

define('m-todo-form', () => {
  const text = signal('');
  const error = signal('');
  const id = uid('nova-tarefa');
  const input = h('input', {
    id,
    class: 'input',
    autocomplete: 'off',
    value: text,
    'aria-invalid': () => (error() ? 'true' : null),
    'aria-describedby': `${id}-erro`,
    oninput: (event) =>
      batch(() => {
        text.set(event.target.value);
        error.set('');
      }),
  });
  const onSubmit = (event) => {
    event.preventDefault();
    const message = addTodo(text());
    batch(() => {
      error.set(message);
      if (!message) text.set('');
    });
    input.focus();
  };
  return h(
    'form',
    { class: 'todo-form', novalidate: true, onsubmit: onSubmit },
    h('label', { for: id }, 'Nova tarefa'),
    h('div', { class: 'todo-form-row' }, input, h('button', { type: 'submit', class: 'button button-primary' }, 'Adicionar')),
    h('p', { id: `${id}-erro`, class: 'field-error', role: 'alert' }, error),
  );
});
