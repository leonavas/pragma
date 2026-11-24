import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/overlay.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/overlay",
  overlays: 1,
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {},
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"e":{"click":[["open"],[0]]}},
    p1: {"o":1},
    p2: {"e":{"click":[["open"],[0]]},"d":1},
    p3: {"b":[[2,["state","locked"],0,"disabled"]],"e":{"click":[["open"],[0]]},"d":1},
    p4: {"o":1,"d":1},
  },
}
export default R
start(R)
