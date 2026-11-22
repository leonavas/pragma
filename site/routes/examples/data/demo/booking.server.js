import { error } from 'pragma/server'

const taken = new Set(['Tue'])

export const actions = {
  async book({ input }) {
    const day = String(input?.day)
    if (taken.has(day)) error(409, day + ' is taken.')
    return { message: 'Booked for ' + day + '.' }
  },
}
