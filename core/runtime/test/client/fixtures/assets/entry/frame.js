import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/frame.js'
import c0 from '/assets/flag.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/frame",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: { "flag": c0 },
  stores: { "cart": [s0, s0p] },
  t: {
    r1: "<li data-b=\"p4\"></li>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"b":[[0,["",["state","on"],"|",["state","n"],"|",["state","label"],"|",["state","who"],""]]]},
    p1: {"x":1,"c":"flag","w":[["on",{"v":false}],["n",{"v":-2.5}],["label",{"v":"a b"}],["who",["state","who"]]]},
    p2: {"t":2,"s":"r1","v":"item","p":["state","items"],"k":["item","id"]},
    p3: {"b":[[6,["state","x"],0,"x"]]},
    p4: {"b":[[0,["",["item","name"],""]]]},
  },
}
export default R
start(R)
