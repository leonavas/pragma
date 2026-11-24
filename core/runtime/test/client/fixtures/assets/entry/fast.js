import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/fast.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/fast",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    f1: "<p id=\"open\" data-b=\"p4\"></p>",
    f2: "<li data-b=\"p6\"><span data-b=\"p5\"></span></li>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"b":[[6,["state","x"],0,"x"],[3,["state","far"],0,"far"],[2,["state","x"],0,"aria-valuenow"]]},
    p1: {"b":[[0,["",["state","label"],""]]]},
    p2: {"t":1,"s":"f1","p":["state","open"],"n":0},
    p3: {"t":2,"s":"f2","v":"item","p":["state","rows"],"k":["item","id"]},
    p4: {"b":[[0,["x = ",["state","x"],""]]]},
    p5: {"b":[[0,["",["item","name"],""]]]},
    p6: {"b":[[3,["item","done"],0,"done"]]},
  },
}
export default R
start(R)
