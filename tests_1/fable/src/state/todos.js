import { computed } from '../lib/reactive.js';
import { persisted } from './persisted.js';

/** @typedef {{ id: string, title: string, done: boolean }} Todo */

export const todos = persisted('mercadinho:tarefas', [
  { id: 't1', title: 'Comprar café', done: false },
  { id: 't2', title: 'Pagar contas', done: true },
  { id: 't3', title: 'Ligar para a farmácia', done: false },
]);

export const totalCount = computed(() => todos().length);
export const doneCount = computed(() => todos().filter((todo) => todo.done).length);
export const pendingCount = computed(() => totalCount() - doneCount());

// randomUUID() exige contexto seguro; isto funciona também em http na rede local.
const newId = () => `t${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

/** Devolve a mensagem de erro, ou '' se o título for válido. */
export function validateTitle(title, ignoreId) {
  const text = title.trim();
  if (!text) return 'Informe um título.';
  if (text.length < 3 || text.length > 80) return 'O título precisa ter de 3 a 80 caracteres.';
  const lower = text.toLowerCase();
  if (todos().some((todo) => todo.id !== ignoreId && todo.title.toLowerCase() === lower)) {
    return 'Já existe uma tarefa com esse título.';
  }
  return '';
}

/** Adiciona a tarefa se for válida. Devolve a mensagem de erro ('' = ok). */
export function addTodo(title) {
  const error = validateTitle(title);
  if (!error) todos.update((list) => [...list, { id: newId(), title: title.trim(), done: false }]);
  return error;
}

/** Renomeia a tarefa se o título for válido. Devolve a mensagem de erro ('' = ok). */
export function renameTodo(id, title) {
  const error = validateTitle(title, id);
  if (!error) todos.update((list) => list.map((todo) => (todo.id === id ? { ...todo, title: title.trim() } : todo)));
  return error;
}

export function toggleTodo(id) {
  todos.update((list) => list.map((todo) => (todo.id === id ? { ...todo, done: !todo.done } : todo)));
}

export function removeTodo(id) {
  todos.update((list) => list.filter((todo) => todo.id !== id));
}

export function clearDone() {
  todos.update((list) => list.filter((todo) => !todo.done));
}
