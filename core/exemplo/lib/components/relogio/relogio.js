const agora = () => new Date().toLocaleTimeString('pt-BR')

export const state = () => ({ hora: agora() })

export function mount(ctx) {
  const t = setInterval(() => ctx.update(s => { s.hora = agora() }), 1000)
  return () => clearInterval(t)
}
