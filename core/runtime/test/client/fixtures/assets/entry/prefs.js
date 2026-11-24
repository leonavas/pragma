import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/prefs.js'
import s0, { persist as s0p } from '/assets/cart.js'
import s1, { persist as s1p } from '/assets/theme.js'

const R = {
  pattern: "/prefs",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p], "theme": [s1, s1p] },
  t: {
    p1: "<li><button data-b=\"p5\"></button></li>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"b":[[0,["",["shared","theme","mode"],""]]]},
    p1: {"e":{"click":[["shared","theme","set"],[{"v":"wide"}]]}},
    p2: {"e":{"click":[["nums"],[{"v":0.5},{"v":1000},{"v":-0.0025},{"v":1},{"v":1},{"v":7}]]}},
    p3: {"b":[[0,["",["state","got"],""]]]},
    p4: {"t":2,"s":"p1","v":"constructor","p":["state","parts"],"k":["constructor","id"]},
    p5: {"b":[[0,["",["constructor","name"],""]]],"e":{"click":[["pick"],[["constructor"]]]}},
  },
}
export default R
start(R)
