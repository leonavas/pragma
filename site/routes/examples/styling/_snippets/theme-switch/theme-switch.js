import { theme, setTheme } from '../../../core/theme.js'

export const state = () => ({
  dark: theme() === 'dark',
  get label() { return this.dark ? 'Light theme' : 'Dark theme' },
  get pressed() { return String(this.dark) },
})

export const actions = {
  toggle(ctx) {
    ctx.$.dark = !ctx.$.dark
    setTheme(ctx.$.dark ? 'dark' : 'light')
  },
}
