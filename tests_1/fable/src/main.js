import { define } from './lib/component.js';
import { h } from './lib/dom.js';
import { routeView } from './lib/router.js';
import './components/app-header.js';
import './pages/painel.js';
import './pages/tarefas.js';
import './pages/loja.js';
import './pages/nao-encontrada.js';

const routes = [
  { path: '/', tag: 'm-page-painel', title: 'Painel · Mercadinho' },
  { path: '/tarefas', tag: 'm-page-tarefas', title: 'Tarefas · Mercadinho' },
  { path: '/loja', tag: 'm-page-loja', title: 'Loja · Mercadinho' },
  { path: '/loja/:id', tag: 'm-page-produto', title: 'Produto · Mercadinho' },
  { path: '*', tag: 'm-page-404', title: 'Página não encontrada · Mercadinho' },
];

// Definido por último: quando <m-app> conecta, todos os componentes já existem.
define('m-app', () => [h('m-app-header'), h('main', { class: 'app-main' }, routeView(routes))]);
