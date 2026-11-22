export const ssr = true

const entries = [{ id: 1, name: 'Ada' }]

export async function load() {
  return { entries }
}

export const actions = {
  async sign({ input }) {
    const typed = String(input?.name ?? '')
    const name = typed.trim().slice(0, 40)
    if (name) entries.unshift({ id: Date.now(), name })
    entries.length = Math.min(entries.length, 5)
    return { entries }
  },
}
