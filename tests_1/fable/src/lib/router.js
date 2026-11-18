// Router sobre a History API. A URL é a fonte: `path` e `search` são signals.
import { h } from './dom.js';
import { batch, computed, signal } from './reactive.js';

export const path = signal(location.pathname);
export const search = signal(location.search);

function sync() {
  batch(() => {
    path.set(location.pathname);
    search.set(location.search);
  });
}

export function navigate(url, { replace = false } = {}) {
  const from = location.pathname;
  history[replace ? 'replaceState' : 'pushState'](null, '', url);
  sync();
  if (location.pathname !== from) scrollTo(0, 0);
}

addEventListener('popstate', sync);

// Links internos navegam sem recarregar. Cliques modificados, target, download,
// outra origem e âncoras da mesma página ficam com o browser.
addEventListener('click', (event) => {
  if (event.defaultPrevented || event.button || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
  const link = event.target.closest?.('a[href]');
  if (!link || link.target || link.hasAttribute('download') || link.origin !== location.origin) return;
  if (link.hash && link.pathname === location.pathname && link.search === location.search) return;
  event.preventDefault();
  if (link.href !== location.href) navigate(link.href);
});

/** Getter reativo de um parâmetro da query string (`null` se ausente). */
export const query = (name) => computed(() => new URLSearchParams(search()).get(name));

/** URL atual com o parâmetro trocado (ou removido, se `value` for vazio). */
export function withQuery(name, value) {
  const params = new URLSearchParams(search());
  if (value) params.set(name, value);
  else params.delete(name);
  const qs = params.toString();
  return path() + (qs ? `?${qs}` : '');
}

/**
 * Região reativa que renderiza a rota atual: use como filho em `h(...)`.
 * Só reage ao `path`; mudar a query não remonta a página.
 * @param {{ path: string, tag: string, title: string }[]} routes
 *   `path` aceita segmentos `:nome`; use `*` na última rota como 404.
 */
export function routeView(routes) {
  const compiled = routes.map((route) => ({
    ...route,
    pattern: new RegExp(`^${route.path.replace('*', '.*').replace(/:(\w+)/g, '(?<$1>[^/]+)')}/?$`),
  }));
  return () => {
    const current = path();
    for (const route of compiled) {
      const match = route.pattern.exec(current);
      if (!match) continue;
      const params = {};
      for (const name in match.groups) params[name] = decodeURIComponent(match.groups[name]);
      document.title = route.title;
      return h(route.tag, { params });
    }
  };
}
