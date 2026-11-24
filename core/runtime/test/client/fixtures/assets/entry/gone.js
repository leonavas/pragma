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
  pattern: "/gone",
  loader: 1,
  use: [branch, list, model, self, vars, persist, net],
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    g1: "<p id=\"sk\">loading</p>",
    z1: "<h1 id=\"gone\">should not render</h1>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"t":1,"s":"g1","p":["page","loading"],"n":0},
    p1: {"t":3,"s":"z1"},
  },
}
export default R
start(R)
