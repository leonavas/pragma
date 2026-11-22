import { THEME_KEY } from './contract.js'

const root = document.documentElement

export const theme = () => root.dataset.theme

export function setTheme(t) {
  root.dataset.theme = t
  try { localStorage.setItem(THEME_KEY, t) } catch {}
}
