import { component, text } from '../lib/component.js';
import { setQuery } from '../lib/router.js';
import { SearchInput, FilterBar } from '../components/common.js';
import { ProductGrid, CartSummary } from '../components/shop.js';
import { loadProducts } from '../state.js';
export const Shop = {
  mount(root, props) {
    const c = component(root, '<div class="page-heading"><div><h1></h1><p class="muted">O essencial para abastecer o seu dia.</p><a class="back-link" href="/loja" hidden>← Voltar para a loja</a></div></div><div class="shop-layout"><section aria-label="Catálogo"><div class="toolbar"><div data-search></div><div data-filter></div></div><div data-grid></div></section><div data-cart></div></div>');
    let search = '';
    const category = () => ['mercearia', 'limpeza', 'hortifruti'].includes(props.url.searchParams.get('categoria')) ? props.url.searchParams.get('categoria') : '';
    const searchProps = () => ({ placeholder: 'Buscar produto…', value: search, onInput: next => { search = next; update(props); } });
    const filterProps = () => ({ label: 'Filtrar produtos', value: category(), options: [['', 'Todas'], ['mercearia', 'Mercearia'], ['limpeza', 'Limpeza'], ['hortifruti', 'Hortifruti']], onChange: next => setQuery(props.navigate, 'categoria', next) });
    const gridProps = () => ({ ...props, search, category: category(), force: props.url.searchParams.get('falhar') === '1', onRetry: () => {
      setQuery(props.navigate, 'falhar', '', true);
      loadProducts();
    } });
    const input = c.own(SearchInput.mount(c.find('[data-search]'), searchProps()));
    const filter = c.own(FilterBar.mount(c.find('[data-filter]'), filterProps()));
    const grid = c.own(ProductGrid.mount(c.find('[data-grid]'), gridProps()));
    const cart = c.own(CartSummary.mount(c.find('[data-cart]'), props));
    function update(p) {
      props = p;
      text(c.find('h1'), p.id ? 'Detalhe do produto' : 'Loja');
      c.find('.back-link').hidden = !p.id;
      c.find('.toolbar').hidden = Boolean(p.id);
      input.update(searchProps());
      filter.update(filterProps());
      grid.update(gridProps());
      cart.update(p);
    }
    update(props);
    // A carga pertence ao store, não à página: uma navegação não duplica o fetch.
    queueMicrotask(() => loadProducts());
    return { update, destroy: c.destroy };
  },
};
