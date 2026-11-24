import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/docs/:a/:b",
  use: [branch, list, model, self, vars, persist, net],
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {},
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
  },
}
export default R
start(R)
