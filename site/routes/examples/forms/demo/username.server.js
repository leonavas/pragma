import { error } from 'pragma/server'

const taken = ['ada', 'linus']

export const actions = {
  async claim({ input }) {
    const name = String(input?.name ?? '').trim()
    if (taken.includes(name.toLowerCase())) {
      error(409, `${name} is taken.`)
    }
    taken.push(name.toLowerCase())
    return { name }
  },
}
