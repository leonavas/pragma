const root = document.documentElement

export const theme = () => root.dataset.theme

export function setTheme(t) {
  root.dataset.theme = t
  try { localStorage.setItem('pragma:theme', t) } catch {}
}
