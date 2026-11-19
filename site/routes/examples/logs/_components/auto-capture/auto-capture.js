let nextFailureId = 1

export const state = () => ({ failures: [] })

export const actions = {
  throwNow() {
    throw new Error('inventory sync crashed')
  },
  async rejectLater() {
    await new Promise(resolve => setTimeout(resolve, 150))
    throw new Error('price refresh timed out')
  },
}

export function mount(ctx) {
  const remember = (kind, message) => ctx.update(current => {
    current.failures = [{ id: nextFailureId++, kind, message }, ...current.failures].slice(0, 4)
  })
  const onError = event => remember('error', event.message)
  const onRejection = event => remember('unhandledrejection', event.reason?.message ?? String(event.reason))
  addEventListener('error', onError)
  addEventListener('unhandledrejection', onRejection)
  return () => {
    removeEventListener('error', onError)
    removeEventListener('unhandledrejection', onRejection)
  }
}
