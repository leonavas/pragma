import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/ssr.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/ssr",
  loader: 1,
  ssr: 1,
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    s1: "<li data-b=\"p2\"></li>",
    z1: "<li>nothing</li>",
    s2: "<p id=\"d\">featured</p>",
    z2: "<p id=\"none\">not featured</p>",
    s4: "<p id=\"nd\">without</p>",
    z3: "<p id=\"cd\">with</p>",
    s3: "<p id=\"open\">open</p>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"b":[[0,["",["data","title"],""]]]},
    p1: {"t":2,"s":"s1","v":"x","p":["data","items"],"k":["x","id"]},
    p2: {"b":[[0,["",["x","name"],""]]]},
    p3: {"t":3,"s":"z1"},
    p4: {"t":1,"s":"s2","p":["data","featured"],"n":0},
    p5: {"t":3,"s":"z2"},
    p6: {"t":1,"s":"s4","p":["data","featured"],"n":1},
    p7: {"t":3,"s":"z3"},
    p8: {"b":[[0,["",["state","n"],""]]]},
    p9: {"e":{"click":[["inc"],[0]]}},
    pa: {"t":1,"s":"s3","p":["state","open"],"n":0},
  },
}
export default R
start(R)
