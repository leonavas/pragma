const agora = () => new Date().toLocaleTimeString('pt-BR')

export const state = () => ({ hora: agora() })

export function mount(ctx) {
  const intervalo = setInterval(() => ctx.update(estado => { estado.hora = agora() }), 1000)
  return () => clearInterval(intervalo)
}
