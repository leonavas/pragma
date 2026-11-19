const now = () => new Date().toLocaleTimeString('en-GB')

export const state = () => ({ time: now() })

export function mount(ctx) {
  const interval = setInterval(() => ctx.update(clock => { clock.time = now() }), 1000)
  return () => clearInterval(interval)
}
