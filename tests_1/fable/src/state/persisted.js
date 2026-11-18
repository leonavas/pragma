import { effect, signal } from '../lib/reactive.js';

/** Signal global cujo valor (JSON) é salvo em localStorage a cada mudança. */
export function persisted(key, initial) {
  let value = initial;
  try {
    const stored = localStorage.getItem(key);
    if (stored !== null) value = JSON.parse(stored);
  } catch {
    // storage indisponível ou conteúdo inválido: segue com o valor inicial
  }
  const state = signal(value);
  effect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(state()));
    } catch {
      // sem persistência (modo privado, cota): o app continua funcionando
    }
  });
  return state;
}
