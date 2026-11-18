import './styles.css';
import { component, text } from './lib/component.js';
import { createRouter } from './lib/router.js';
import { store, applyStateEffects } from './state.js';
import { AppHeader } from './components/common.js';
import { Dashboard } from './pages/dashboard.js';
import { Tasks } from './pages/tasks.js';
import { Shop } from './pages/shop.js';
const app = component(document.querySelector('#app'), '<div data-header></div><main id="conteudo" tabindex="-1"></main><footer>Mercadinho · Seu cotidiano, simplificado.</footer>');
let url = new URL(location.href);
let page;
let pageKey;
const header = app.own(AppHeader.mount(app.find('[data-header]'), { state: store.getState(), path: url.pathname }));
const NotFound = {
  mount(root) {
    const c = component(root, '<section class="not-found"><p class="muted">404</p><h1>Essa página não está por aqui.</h1><p>Volte ao painel para continuar.</p><a href="/">Voltar para o início</a></section>');
    return { update: () => {}, destroy: c.destroy };
  },
};
function route(path) {
  if (path === '/') return { view: Dashboard, title: 'Painel' };
  if (path === '/tarefas') return { view: Tasks, title: 'Tarefas' };
  if (path === '/loja') return { view: Shop, title: 'Loja' };
  const match = path.match(/^\/loja\/([^/]+)$/);
  if (match) return { view: Shop, title: 'Produto', id: match[1] };
  return { view: NotFound, title: 'Página não encontrada' };
}
function render() {
  const state = store.getState();
  const match = route(url.pathname);
  const props = { state, url, navigate: router.navigate, id: match.id };
  applyStateEffects(state);
  header.update({ state, path: url.pathname });
  if (pageKey !== url.pathname) {
    page?.destroy();
    pageKey = url.pathname;
    page = match.view.mount(app.find('main'), props);
    app.find('main').focus({ preventScroll: true });
    window.scrollTo(0, 0);
  } else page.update(props);
  const product = match.id && state.products.data.find(p => p.id === match.id);
  text(document.querySelector('title'), `${product?.nome || match.title} · Mercadinho`);
}
const router = createRouter(next => { url = next; render(); });
app.cleanup(store.subscribe(render));
app.cleanup(router.destroy);
app.cleanup(() => page?.destroy());
router.start();
