import { theme, setTheme } from '../../../core/theme.js'

export const state = () => ({
  escuro: theme() === 'dark',
  get rotulo() { return this.escuro ? 'Tema claro' : 'Tema escuro' },
  get pressionado() { return String(this.escuro) },
})

export const actions = {
  alternar(ctx) {
    ctx.$.escuro = !ctx.$.escuro
    setTheme(ctx.$.escuro ? 'dark' : 'light')
  },
}
