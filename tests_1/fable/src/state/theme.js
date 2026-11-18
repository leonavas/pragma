import { effect, signal } from '../lib/reactive.js';

const KEY = 'mercadinho:tema';
const root = document.documentElement;

// O index.html já aplicou o tema salvo antes do primeiro paint.
export const theme = signal(root.dataset.theme === 'dark' ? 'dark' : 'light');

export function toggleTheme() {
  theme.update((current) => (current === 'dark' ? 'light' : 'dark'));
}

effect(() => {
  root.dataset.theme = theme(); // dom-ok: effect global, elemento fora do app
  try {
    localStorage.setItem(KEY, theme());
  } catch {
    // sem persistência: o tema vale só para esta visita
  }
});
