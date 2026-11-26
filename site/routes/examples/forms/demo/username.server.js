import { error } from 'pragma/server'

const taken = ['nova', 'zelda']

export const actions = {
  async claim({ input }) {
    const name = String(input?.name ?? '').trim()
    if (taken.includes(name.toLowerCase())) {
      error(409, `${name} is already on the leaderboard.`)
    }
    taken.push(name.toLowerCase())
    return { name }
  },
}
