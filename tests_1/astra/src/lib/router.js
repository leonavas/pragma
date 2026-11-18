export function createRouter(resolve) {
  const scope = new AbortController();
  function navigate(href, replace = false) {
    history[replace ? 'replaceState' : 'pushState'](null, '', href);
    resolve(new URL(location.href));
  }
  document.addEventListener('click', event => {
    const link = event.target.closest('a[href]');
    if (!link || event.defaultPrevented || event.button !== 0 ||
        event.metaKey || event.ctrlKey || event.shiftKey || event.altKey ||
        link.hasAttribute('download') || (link.target && link.target !== '_self')) return;
    const url = new URL(link.href);
    if (url.origin !== location.origin || url.hash) return;
    event.preventDefault();
    navigate(url.href);
  }, { signal: scope.signal });
  window.addEventListener('popstate', () => resolve(new URL(location.href)), { signal: scope.signal });
  return { navigate, start: () => resolve(new URL(location.href)), destroy: () => scope.abort() };
}
export function setQuery(navigate, key, value, replace = false) {
  const url = new URL(location.href);
  if (value) url.searchParams.set(key, value);
  else url.searchParams.delete(key);
  navigate(url.href, replace);
}
