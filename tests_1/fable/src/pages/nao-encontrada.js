import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';

define('m-page-404', () => [
  h('h1', null, 'Página não encontrada'),
  h('p', null, 'O endereço não existe no Mercadinho.'),
  h('p', null, h('a', { href: '/' }, 'Voltar para o início')),
]);
