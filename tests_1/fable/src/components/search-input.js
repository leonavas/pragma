import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { uid } from '../format.js';

// Input controlado. Props: label, placeholder, value (getter), onInput(texto).
define('m-search-input', ({ label, placeholder, value, onInput }) => {
  const id = uid('busca');
  return [
    h('label', { class: 'sr-only', for: id }, label),
    h('input', { id, type: 'search', class: 'input', placeholder, value, oninput: (event) => onInput(event.target.value) }),
  ];
});
