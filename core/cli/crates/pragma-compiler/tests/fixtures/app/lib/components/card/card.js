import { shared } from 'pragma'

export let heading = ''
export let featured = false

function buy() {
  shared.cart.total += 1
}
