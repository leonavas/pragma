import { createStore } from './lib/store.js';
const key = 'mercadinho-astra-v1';
const defaults = [
  { id: 'cafe', title: 'Comprar café', done: false },
  { id: 'contas', title: 'Pagar contas', done: true },
  { id: 'farmacia', title: 'Ligar para a farmácia', done: false },
];
function restore() {
  try { return JSON.parse(localStorage.getItem(key)) || {}; } catch { return {}; }
}
const saved = restore();
const validTodo = t => t && typeof t.id === 'string' && typeof t.title === 'string' && typeof t.done === 'boolean';
const validCart = p => p && typeof p.id === 'string' && typeof p.nome === 'string' &&
  Number.isFinite(p.preco) && Number.isInteger(p.estoque) && Number.isInteger(p.qty) && p.qty > 0 && p.qty <= p.estoque;
export const normalize = value => value.trim().toLocaleLowerCase('pt-BR');
export function todoError(todos, title, except) {
  if (!title.trim()) return 'Digite o título da tarefa.';
  if (title.trim().length < 3 || title.trim().length > 80) return 'Use entre 3 e 80 caracteres.';
  if (todos.some(t => t.id !== except && normalize(t.title) === normalize(title))) return 'Esta tarefa já existe.';
  return '';
}
function reduce(state, action) {
  switch (action.type) {
    case 'theme': return { ...state, theme: state.theme === 'dark' ? 'light' : 'dark' };
    case 'todo-add':
      if (todoError(state.todos, action.title)) return state;
      return { ...state, todos: [...state.todos, { id: crypto.randomUUID(), title: action.title.trim(), done: false }] };
    case 'todo-edit':
      if (todoError(state.todos, action.title, action.id)) return state;
      return { ...state, todos: state.todos.map(t => t.id === action.id ? { ...t, title: action.title.trim() } : t) };
    case 'todo-toggle': return { ...state, todos: state.todos.map(t => t.id === action.id ? { ...t, done: !t.done } : t) };
    case 'todo-remove': return { ...state, todos: state.todos.filter(t => t.id !== action.id) };
    case 'todo-clear': return { ...state, todos: state.todos.filter(t => !t.done) };
    case 'cart-add': {
      const product = state.products.data.find(p => p.id === action.id);
      if (!product || !product.estoque) return state;
      const current = state.cart.find(p => p.id === action.id);
      const qty = Math.min(product.estoque, (current?.qty || 0) + action.qty);
      if (!Number.isInteger(qty) || qty < 1) return state;
      return { ...state, cart: current ? state.cart.map(p => p.id === action.id ? { ...p, qty } : p) : [...state.cart, { ...product, qty }] };
    }
    case 'cart-quantity': return { ...state, cart: state.cart.map(p => p.id === action.id ? { ...p, qty: Math.max(1, Math.min(p.estoque, p.qty + action.delta)) } : p) };
    case 'cart-remove': return { ...state, cart: state.cart.filter(p => p.id !== action.id) };
    case 'cart-clear': return { ...state, cart: [] };
    case 'products': return { ...state, products: action.value };
    default: return state;
  }
}
export const store = createStore({
  todos: Array.isArray(saved.todos) && saved.todos.every(validTodo) ? saved.todos : defaults,
  cart: Array.isArray(saved.cart) && saved.cart.every(validCart) ? saved.cart : [],
  theme: saved.theme === 'dark' ? 'dark' : 'light',
  products: { status: 'idle', data: [] },
}, reduce);
export const dispatch = store.dispatch;
export const cartCount = state => state.cart.reduce((sum, p) => sum + p.qty, 0);
export const cartTotal = state => state.cart.reduce((sum, p) => sum + Math.round(p.preco * 100) * p.qty, 0) / 100;
export const pendingCount = state => state.todos.filter(t => !t.done).length;
export const money = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' }).format;
let previous;
export function applyStateEffects(state) {
  document.documentElement.dataset.theme = state.theme;
  if (!previous || previous.todos !== state.todos || previous.cart !== state.cart || previous.theme !== state.theme) {
    try { localStorage.setItem(key, JSON.stringify({ todos: state.todos, cart: state.cart, theme: state.theme })); } catch { /* App segue utilizável se o armazenamento estiver indisponível. */ }
  }
  previous = state;
}
let request;
export function loadProducts() {
  if (store.getState().products.status === 'ready') return Promise.resolve();
  if (request) return request;
  dispatch({ type: 'products', value: { status: 'loading', data: [] } });
  request = (async () => {
    try {
      const [response] = await Promise.all([fetch('/produtos.json'), new Promise(resolve => setTimeout(resolve, 400))]);
      if (!response.ok) throw new Error('Falha no catálogo');
      const data = await response.json();
      dispatch({ type: 'products', value: { status: 'ready', data } });
    } catch {
      dispatch({ type: 'products', value: { status: 'error', data: [] } });
    } finally { request = null; }
  })();
  return request;
}
