import { component, text, value } from '../lib/component.js';
import { keyedList } from '../lib/list.js';
import { dispatch, todoError } from '../state.js';
export const TodoForm = {
  mount(root, props) {
    const c = component(root, '<form novalidate><label for="new-todo">Uma tarefa a menos para lembrar</label><div class="form-line"><input id="new-todo" name="title" placeholder="O que precisa ser feito?" required minlength="3" maxlength="80" aria-describedby="new-todo-error" autocomplete="off"><button>Adicionar</button></div><p id="new-todo-error" class="error" role="alert"></p></form>');
    const input = c.find('input');
    c.on(c.find('form'), 'submit', event => {
      event.preventDefault();
      const error = todoError(props.state.todos, input.value);
      text(c.find('.error'), error);
      input.setAttribute('aria-invalid', String(Boolean(error)));
      if (error) return;
      dispatch({ type: 'todo-add', title: input.value });
      input.value = '';
      input.focus();
    });
    return { update: p => { props = p; }, destroy: c.destroy };
  },
};
const TodoItem = {
  mount(root, props) {
    const c = component(root, '<div class="todo-row"><input type="checkbox"><button class="todo-title" type="button"></button><input class="todo-edit" type="text" minlength="3" maxlength="80" hidden><button class="quiet edit-action" type="button">Editar</button><button class="quiet remove-action" type="button">Remover</button></div><p class="error" role="alert"></p>');
    let editing = false;
    const title = c.find('.todo-title');
    const input = c.find('.todo-edit');
    const check = c.find('[type="checkbox"]');
    const errorNode = c.find('.error');
    errorNode.id = `edit-error-${props.id}`;
    input.setAttribute('aria-describedby', errorNode.id);
    function edit() {
      editing = true;
      value(input, props.title);
      update(props);
      input.focus();
      input.select();
    }
    function finish() {
      editing = false;
      text(errorNode, '');
      update(props);
      title.focus();
    }
    c.on(title, 'dblclick', edit);
    c.on(c.find('.edit-action'), 'click', edit);
    c.on(check, 'change', () => dispatch({ type: 'todo-toggle', id: props.id }));
    c.on(c.find('.remove-action'), 'click', () => dispatch({ type: 'todo-remove', id: props.id }));
    c.on(input, 'keydown', event => {
      if (event.key === 'Escape') { event.preventDefault(); finish(); }
      if (event.key === 'Enter') {
        event.preventDefault();
        const error = todoError(props.todos, input.value, props.id);
        text(errorNode, error);
        input.setAttribute('aria-invalid', String(Boolean(error)));
        if (error) return;
        const action = { type: 'todo-edit', id: props.id, title: input.value };
        finish();
        dispatch(action);
      }
    });
    function update(p) {
      props = p;
      text(title, p.title);
      title.hidden = editing;
      title.classList.toggle('done', p.done);
      check.checked = p.done;
      check.setAttribute('aria-label', `Concluir ${p.title}`);
      input.hidden = !editing;
      input.setAttribute('aria-label', `Editar ${p.title}`);
      c.find('.edit-action').hidden = editing;
      c.find('.edit-action').setAttribute('aria-label', `Editar ${p.title}`);
      c.find('.remove-action').setAttribute('aria-label', `Remover ${p.title}`);
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const TodoList = {
  mount(root, props) {
    const c = component(root, '<ul class="todo-list" aria-label="Lista de tarefas"></ul><p class="empty" hidden>Nenhuma tarefa</p>');
    const list = c.own(keyedList(c.find('ul'), TodoItem.mount, 'li'));
    function update(p) {
      list.update(p.items.map(item => ({ ...item, todos: p.state.todos })));
      c.find('.empty').hidden = p.items.length !== 0;
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
