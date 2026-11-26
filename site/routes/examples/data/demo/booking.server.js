import { error } from 'pragma/server'

const taken = new Set(['1999'])

export const actions = {
  async book({ input }) {
    const year = String(input?.year)
    if (taken.has(year)) error(409, year + ' is booked solid for Y2K.')
    return { message: 'Time slot reserved for ' + year + '.' }
  },
}
