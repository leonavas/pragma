import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/template.js'
import c0 from '/assets/inner.js'
import s0 from '/assets/basket.js'

const R = {
  pattern: "/template",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: { "inner": c0 },
  stores: { "basket": [s0] },
  t: {
    t1: "<li data-b=\"pn\"><label data-b=\"pl\"><input type=\"checkbox\" data-b=\"pk\"> <!--{--><!--}--></label><button class=\"rename\" data-b=\"pm\">rename</button></li>",
    t8: "<li><button class=\"drop\" data-b=\"po\"></button></li>",
    t9: "<span data-b=\"pp\"></span>",
  },
  b: {
    l0: {"x":2},
    p0: {"b":[[0,["",["state","count"],""]]]},
    p1: {"b":[[0,["Hello ",["state","name"],", you have ",["state","count"]," items {literal} & more"]]]},
    p2: {"b":[[4,["/u/",["state","id"],"?tab=1"],0,"href"],[4,["Open ",["state","name"],""],0,"title"]]},
    p3: {"b":[[5,["state","total"],0,0],[5,["state","name"],1,0]]},
    p4: {"b":[[5,["state","name"],0,0]]},
    p5: {"b":[[0,["",["state","pending","length"]," pending"]]]},
    p6: {"t":2,"s":"t1","v":"task","p":["state","tasks"],"k":["task","id"]},
    p7: {"e":{"click":[["record"],[0,{"v":"a, b"},{"v":-2.5},{"v":true},{"v":null},["state","name"]]]}},
    p8: {"b":[[0,["",["state","log"],""]]]},
    p9: {"e":{"input":[["search"],[0]]}},
    pa: {"b":[[0,["",["state","query"],""]]]},
    pb: {"e":{"click":[["locate"],[1,0]]}},
    pc: {"b":[[0,["",["state","spot"],""]]]},
    pd: {"e":{"click":[["saveTwice"],[0]]}},
    pe: {"b":[[0,["",["state","saves"],""]]]},
    pf: {"e":{"click":[["name"],[0]]}},
    pg: {"t":2,"s":"t8","v":"item","p":["shared","basket","items"],"k":["item","id"]},
    ph: {"t":2,"s":"t9","v":"line","p":["state","tasks"],"k":["line","id"]},
    pi: {"b":[[0,["",["state","name"],""]]]},
    pj: {"x":1,"c":"inner","b":[[5,["state","name"],0,1]]},
    pk: {"b":[[2,["task","done"],0,"checked"]],"e":{"change":[["toggle"],[["task"]]]}},
    pl: {"b":[[5,["task","title"],0,0]]},
    pm: {"e":{"click":[["rename"],[["task"],{"v":"Renamed"}]]}},
    pn: {"b":[[3,["task","done"],0,"done"]]},
    po: {"b":[[0,["",["item","name"],""]]],"e":{"click":[["shared","basket","remove"],[["item","id"]]]}},
    pp: {"b":[[2,["line","id"],0,"data-n"],[0,["",["line","title"],";"]]]},
  },
}
export default R
start(R)
