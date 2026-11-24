import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/slot.js'
import c0 from '/assets/box.js'
import c1 from '/assets/counter.js'
import s0, { persist as s0p } from '/assets/account.js'

const R = {
  pattern: "/slot",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: { "box": c0, "counter": c1 },
  stores: { "account": [s0, s0p] },
  t: {
    s1: "<b id=\"page-if\" data-b=\"p0\"></b>",
    s2: "<li>\n        <div data-c=\"box\" data-b=\"pj\">\n          <output class=\"n\" data-b=\"p2\"></output><span class=\"max\" data-b=\"ph\"></span>\n          <button class=\"item-btn\" data-b=\"pi\"></button>\n        </div>\n      </li>",
    s3: "<input id=\"edit\" autofocus>",
  },
  b: {
    l0: {"b":[[0,["",["shared","account","total"],""]]]},
    l1: {"x":2},
    p0: {"b":[[0,["",["state","title"],""]]]},
    p1: {"e":{"click":[["inc"],[0]]}},
    p2: {"b":[[0,["",["state","n"],""]]]},
    p3: {"u":1,"e":{"click":[["page"],[0]]}},
    p4: {"u":1,"b":[[0,["",["state","title"],""]]]},
    p5: {"u":1,"m":["state","field"]},
    p6: {"t":1,"s":"s1","p":["state","visible"],"n":0,"u":1},
    p7: {"u":2,"e":{"click":[["page"],[0]]}},
    p8: {"u":1,"e":{"click":[["inc"],[0]]}},
    p9: {"x":1,"c":"counter"},
    pa: {"x":1,"c":"box","e":{"click":[["box"],[0]]},"p":{"initial":"5"}},
    pb: {"t":2,"s":"s2","v":"item","p":["state","items"],"k":["item","id"]},
    pc: {"b":[[0,["",["state","field"],""]]]},
    pd: {"b":[[0,["",["state","picked"],""]]]},
    pe: {"b":[[0,["",["state","double"],""]]]},
    pf: {"b":[[2,["state","marked"],0,"checked"]]},
    pg: {"t":1,"s":"s3","p":["state","editing"],"n":0},
    ph: {"b":[[0,["",["state","max"],""]]]},
    pi: {"u":1,"b":[[0,["",["item","name"],""]]],"e":{"click":[["pick"],[["item"]]]}},
    pj: {"x":1,"c":"box","w":[["max",["item","max"]],["row",["item"]]],"p":{"initial":"1"}},
  },
}
export default R
start(R)
