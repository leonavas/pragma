import { component } from '../lib/component.js';
import { setQuery } from '../lib/router.js';
import { SearchInput, FilterBar, StatCard, Modal, paragraph } from '../components/common.js';
import { TodoForm, TodoList } from '../components/todos.js';
import { normalize, dispatch } from '../state.js';
export const Tasks = {
  mount(root, props) {
    const c = component(root, '<div class="page-heading"><div><h1>Tarefas</h1><p class="muted">Tire da cabeça. Coloque na lista.</p></div><div data-clear></div></div><div class="task-layout"><section class="surface"><div data-form></div><div class="toolbar"><div data-search></div><div data-filter></div></div><div data-list></div></section><div data-stat></div></div>');
    let search = '';
    const searchProps = () => ({ placeholder: 'Buscar tarefa…', value: search, onInput: next => { search = next; update(props); } });
    const filterProps = () => ({
      label: 'Filtrar tarefas', value: ['pendentes', 'concluidas'].includes(props.url.searchParams.get('filtro')) ? props.url.searchParams.get('filtro') : '',
      options: [['', 'Todas'], ['pendentes', 'Pendentes'], ['concluidas', 'Concluídas']],
      onChange: next => setQuery(props.navigate, 'filtro', next),
    });
    const input = c.own(SearchInput.mount(c.find('[data-search]'), searchProps()));
    const filter = c.own(FilterBar.mount(c.find('[data-filter]'), filterProps()));
    const form = c.own(TodoForm.mount(c.find('[data-form]'), props));
    const list = c.own(TodoList.mount(c.find('[data-list]'), { ...props, items: [] }));
    const stat = c.own(StatCard.mount(c.find('[data-stat]'), {}));
    c.own(Modal.mount(c.find('[data-clear]'), {
      trigger: 'Limpar concluídas', title: 'Limpar tarefas concluídas?',
      content: paragraph('As tarefas já concluídas serão removidas da lista. As pendentes ficam como estão.'),
      onConfirm: () => dispatch({ type: 'todo-clear' }),
    }));
    function update(p) {
      props = p;
      const active = filterProps().value;
      const todos = p.state.todos;
      const items = todos.filter(t => normalize(t.title).includes(normalize(search)) && (!active || (active === 'concluidas' ? t.done : !t.done)));
      input.update(searchProps());
      filter.update(filterProps());
      form.update(p);
      list.update({ ...p, items });
      const done = todos.filter(t => t.done).length;
      stat.update({ title: 'Concluídas', value: `${done} de ${todos.length}`, hint: 'Cada tarefa conta.', tone: done === todos.length && done > 0 ? 'ok' : 'neutro' });
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
