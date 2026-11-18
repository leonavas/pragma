import { define } from '../lib/component.js';
import { h, read } from '../lib/dom.js';
import { effect } from '../lib/reactive.js';
import { uid } from '../format.js';

// Props: title, open (getter), onClose(). O conteúdo vem dos filhos.
// O pai é dono do estado: o modal só avisa que fechou (Esc, Fechar ou fundo).
// O <dialog> nativo cuida do Esc, da armadilha de foco e de devolver o foco.
define('m-modal', ({ title, open, onClose }, children) => {
  const titleId = uid('modal');
  const dialog = h(
    'dialog',
    {
      class: 'modal',
      'aria-labelledby': titleId,
      onclose: () => onClose?.(),
      onclick: (event) => {
        if (event.target === dialog) dialog.close(); // clique no fundo
      },
    },
    h(
      'div',
      { class: 'modal-box' },
      h(
        'header',
        { class: 'modal-header' },
        h('h2', { id: titleId }, title),
        h('button', { type: 'button', class: 'button', onclick: () => dialog.close() }, 'Fechar'),
      ),
      h('div', { class: 'modal-body' }, children),
    ),
  );
  effect(() => {
    if (!read(open)) dialog.close();
    else if (dialog.isConnected && !dialog.open) dialog.showModal();
  });
  return dialog;
});
