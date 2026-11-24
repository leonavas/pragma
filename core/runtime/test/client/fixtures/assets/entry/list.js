import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/list.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/list",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    i1: "<li data-b=\"p2\"><span data-b=\"p1\"></span></li>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"t":2,"s":"i1","v":"item","p":["state","items"],"k":["item","id"]},
    p1: {"b":[[0,["",["item","name"],""]]]},
    p2: {"b":[[3,["item","done"],0,"done"]]},
  },
}
export default R
start(R)
