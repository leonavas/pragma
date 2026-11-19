import { format } from '../utils/fmt.js'
import { shared, log } from 'pragma'

let open = false
let items = []

function toggle() {
  open = !open
  log.info(format(shared.cart.total))
}
