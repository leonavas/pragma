import { component, text, value } from '../lib/component.js';
import { cartCount, dispatch } from '../state.js';
let serial = 0;
export const Counter = {
  mount(root, props) {
    const c = component(root, '<div class="counter"><span class="counter-label"></span><div class="stepper"><button type="button" data-minus>−</button><output></output><button type="button" data-plus>+</button></div></div>');
    let count = props.initial;
    function update(next) {
      props = next;
      const min = props.min ?? -Infinity;
      const max = Math.max(min, props.max ?? Infinity);
      count = Math.max(min, Math.min(max, count));
      text(c.find('.counter-label'), props.label);
      text(c.find('output'), count);
      c.find('output').setAttribute('aria-label', props.label);
      c.find('[data-minus]').disabled = props.disabled || count <= min;
      c.find('[data-plus]').disabled = props.disabled || count >= max;
      c.find('[data-minus]').setAttribute('aria-label', `Diminuir ${props.label}`);
      c.find('[data-plus]').setAttribute('aria-label', `Aumentar ${props.label}`);
    }
    for (const [selector, sign] of [['[data-minus]', -1], ['[data-plus]', 1]]) {
      c.on(c.find(selector), 'click', () => {
        count += props.step * sign;
        update(props);
        props.onChange?.(count);
      });
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const StatCard = {
  mount(root, props) {
    const c = component(root, '<article class="stat"><h2></h2><p class="stat-value"></p><p class="muted hint"></p></article>');
    function update(p) {
      text(c.find('h2'), p.title);
      text(c.find('.stat-value'), p.value);
      text(c.find('.hint'), p.hint || '');
      c.find('.hint').hidden = !p.hint;
      c.find('article').dataset.tone = p.tone;
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const Clock = {
  mount(root) {
    const c = component(root, '<div class="clock"><span>Agora, por aqui</span><time></time></div>');
    const format = new Intl.DateTimeFormat('pt-BR', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
    function update() {
      const now = new Date();
      text(c.find('time'), format.format(now));
      c.find('time').dateTime = now.toISOString();
    }
    const timer = setInterval(update, 1000);
    c.cleanup(() => clearInterval(timer));
    update();
    return { update: () => {}, destroy: c.destroy };
  },
};
export const ThemeToggle = {
  mount(root, props) {
    const c = component(root, '<button type="button" class="secondary"></button>');
    c.on(c.find('button'), 'click', () => dispatch({ type: 'theme' }));
    function update(p) {
      text(c.find('button'), p.theme === 'dark' ? 'Usar tema claro' : 'Usar tema escuro');
      c.find('button').setAttribute('aria-pressed', String(p.theme === 'dark'));
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const SearchInput = {
  mount(root, props) {
    const c = component(root, '<label class="search"><span class="sr-only"></span><input type="search" autocomplete="off"></label>');
    const input = c.find('input');
    c.on(input, 'input', () => props.onInput(input.value));
    function update(p) {
      props = p;
      text(c.find('span'), p.placeholder.replace('…', ''));
      input.placeholder = p.placeholder;
      value(input, p.value);
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const FilterBar = {
  mount(root, props) {
    const c = component(root, '<div class="filters" role="group"></div>');
    const buttons = props.options.map(([key, label]) => {
      const button = document.createElement('button');
      button.type = 'button';
      text(button, label);
      c.on(button, 'click', () => props.onChange(key));
      c.find('.filters').append(button);
      return [key, button];
    });
    function update(p) {
      props = p;
      c.find('.filters').setAttribute('aria-label', p.label);
      for (const [key, button] of buttons) button.setAttribute('aria-pressed', String(key === p.value));
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
// O conteúdo chega como função de montagem, nunca como HTML fornecido pelo usuário.
export const Modal = {
  mount(root, props) {
    const c = component(root, '<button type="button" class="secondary" data-open></button><dialog><h2></h2><div data-content></div><div class="dialog-actions"><button type="button" class="secondary" data-close></button><button type="button" data-confirm>Confirmar</button></div></dialog>');
    const dialog = c.find('dialog');
    const opener = c.find('[data-open]');
    const title = c.find('h2');
    title.id = `modal-${++serial}`;
    dialog.setAttribute('aria-labelledby', title.id);
    if (props.content) c.own(props.content(c.find('[data-content]')));
    function close(reason) {
      dialog.close();
      opener.focus();
      props.onClose?.(reason);
    }
    c.on(opener, 'click', () => dialog.showModal());
    c.on(c.find('[data-close]'), 'click', () => close('cancel'));
    c.on(c.find('[data-confirm]'), 'click', () => { close('confirm'); props.onConfirm?.(); });
    c.on(dialog, 'cancel', event => { event.preventDefault(); close('cancel'); });
    c.on(dialog, 'click', event => {
      const rect = dialog.getBoundingClientRect();
      if (event.target === dialog && (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom)) close('cancel');
    });
    c.cleanup(() => { if (dialog.open) dialog.close(); });
    function update(p) {
      props = p;
      text(opener, p.trigger);
      text(title, p.title);
      text(c.find('[data-close]'), p.onConfirm ? 'Cancelar' : 'Fechar');
      c.find('[data-confirm]').hidden = !p.onConfirm;
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export function paragraph(content) {
  return root => {
    const node = document.createElement('p');
    text(node, content);
    root.append(node);
    return { update: () => {}, destroy: () => node.remove() };
  };
}
export const Tabs = {
  mount(root, props) {
    const c = component(root, '<div class="tabs"><div role="tablist" aria-label="Conheça o painel"></div><div data-panels></div></div>');
    let selected = 0;
    const tabs = props.items.map((item, index) => {
      const tab = document.createElement('button');
      const panel = document.createElement('section');
      const id = `tab-${++serial}`;
      tab.type = 'button';
      tab.id = id;
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-controls', `${id}-panel`);
      text(tab, item.label);
      panel.id = `${id}-panel`;
      panel.setAttribute('role', 'tabpanel');
      panel.setAttribute('aria-labelledby', id);
      panel.tabIndex = 0;
      c.find('[role="tablist"]').append(tab);
      c.find('[data-panels]').append(panel);
      c.own(item.mount(panel));
      c.on(tab, 'click', () => select(index));
      c.on(tab, 'keydown', event => {
        let next;
        if (event.key === 'ArrowRight') next = (index + 1) % props.items.length;
        if (event.key === 'ArrowLeft') next = (index - 1 + props.items.length) % props.items.length;
        if (event.key === 'Home') next = 0;
        if (event.key === 'End') next = props.items.length - 1;
        if (next !== undefined) { event.preventDefault(); select(next); tabs[next].tab.focus(); }
      });
      return { tab, panel };
    });
    function select(index) {
      selected = index;
      tabs.forEach(({ tab, panel }, i) => {
        tab.setAttribute('aria-selected', String(i === selected));
        tab.tabIndex = i === selected ? 0 : -1;
        panel.hidden = i !== selected;
      });
    }
    select(0);
    return { update: () => {}, destroy: c.destroy };
  },
};
export const NavLink = {
  mount(root, props) {
    const c = component(root, '<a></a>');
    const a = c.find('a');
    a.href = props.href;
    text(a, props.label);
    function update({ path }) {
      const active = props.href === '/' ? path === '/' : path === props.href || path.startsWith(`${props.href}/`);
      if (active) a.setAttribute('aria-current', 'page'); else a.removeAttribute('aria-current');
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const CartBadge = {
  mount(root, props) {
    const c = component(root, '<a class="cart-badge" href="/loja">Carrinho <span aria-live="polite"></span></a>');
    function update(p) { text(c.find('span'), cartCount(p.state)); }
    update(props);
    return { update, destroy: c.destroy };
  },
};
export const AppHeader = {
  mount(root, props) {
    const c = component(root, '<header class="app-header"><a href="/" class="brand">Mercadinho<span>O dia a dia, em ordem.</span></a><nav aria-label="Navegação principal"><span data-nav></span><span data-nav></span><span data-nav></span></nav><div data-cart></div></header>');
    const slots = root.querySelectorAll('[data-nav]');
    const links = [['/', 'Painel'], ['/tarefas', 'Tarefas'], ['/loja', 'Loja']].map(([href, label], i) => c.own(NavLink.mount(slots[i], { href, label, path: props.path })));
    const badge = c.own(CartBadge.mount(c.find('[data-cart]'), props));
    function update(p) { links.forEach(link => link.update(p)); badge.update(p); }
    return { update, destroy: c.destroy };
  },
};
