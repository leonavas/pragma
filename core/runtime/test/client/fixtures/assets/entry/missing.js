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
  pattern: "/missing",
  loader: 1,
  use: [branch, list, model, self, vars, persist, net],
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    x1: "<p id=\"sk\">loading</p>",
    z1: "<p id=\"error\"><span id=\"status\" data-b=\"p3\"></span> <span id=\"code\" data-b=\"p4\"></span> <span id=\"message\" data-b=\"p5\"></span></p>",
    z2: "<h1 id=\"found\">found</h1>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"t":1,"s":"x1","p":["page","loading"],"n":0},
    p1: {"t":4,"s":"z1","p":["page","error"],"n":0},
    p2: {"t":3,"s":"z2"},
    p3: {"b":[[0,["",["page","error","status"],""]]]},
    p4: {"b":[[0,["",["page","error","code"],""]]]},
    p5: {"b":[[0,["",["page","error","message"],""]]]},
  },
}
export default R
start(R)
