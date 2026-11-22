import { redirect } from 'pragma/server'

let lastId = 100

export const actions = {
  async place() {
    lastId += 1
    redirect(303, '/examples/data/demo/done?id=' + lastId)
  },
}
