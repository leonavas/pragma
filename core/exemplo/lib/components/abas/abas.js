const ABAS = ['a', 'b', 'c']
const SETAS = { ArrowRight: 1, ArrowLeft: -1 }
const por = f => Object.fromEntries(ABAS.map(k => [k, f(k)]))

export const state = () => ({
  aba: 'a',
  get ativa() { return por(k => k === this.aba) },
  get selecionada() { return por(k => String(k === this.aba)) },
  get tabindex() { return por(k => (k === this.aba ? 0 : -1)) },
})

export const actions = {
  ir(ctx) { ctx.state.aba = ctx.el.value },
  tecla(ctx) {
    const passo = SETAS[ctx.event.key]
    if (!passo) return
    const i = (ABAS.indexOf(ctx.state.aba) + passo + ABAS.length) % ABAS.length
    ctx.state.aba = ABAS[i]
    ctx.el.children[i].focus()
  },
}
