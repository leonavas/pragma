import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import motion from '/core/motion.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/motion.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/motion",
  use: [branch, list, model, motion, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    m1: "<p id=\"on\">on</p>",
    z1: "<p id=\"off\">off</p>",
    m2: "<li data-b=\"p4\"></li>",
    m3: "<p id=\"plain\" class=\"spin\">plain</p>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"t":1,"s":"m1","p":["state","on"],"n":0},
    p1: {"t":3,"s":"z1"},
    p2: {"t":2,"s":"m2","v":"i","p":["state","items"],"k":["i"]},
    p3: {"t":1,"s":"m3","p":["state","plain"],"n":0},
    p4: {"b":[[0,["",["i"],""]]]},
  },
}
export default R
start(R)
