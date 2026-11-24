import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/menu.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/menu",
  overlays: 1,
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    b1: "<div><button id=\"open-m2\" data-open=\"m2\" data-b=\"p1\">open</button><dialog id=\"m2\"><p>menu</p><button data-close value=\"ok\" data-b=\"p1\">close</button></dialog></div>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"t":1,"s":"b1","p":["state","show"],"n":0},
    p1: {"o":1},
  },
}
export default R
start(R)
