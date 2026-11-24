import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/directives.js'
import c0 from '/assets/counter.js'
import s0, { persist as s0p } from '/assets/cart.js'

const R = {
  pattern: "/directives",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: { "counter": c0 },
  stores: { "cart": [s0, s0p] },
  t: {
    t1: "<p id=\"yes\">yes</p>",
    z1: "<p id=\"no\">no</p>",
    t3: "<b class=\"ok\">ok</b>",
    t4: "<i class=\"tag\" data-b=\"pv\"></i>",
    z2: "<i class=\"none\">no tags</i>",
    t2: "<li>\n        <span class=\"n\" data-b=\"pw\"></span>\n        <!--p:px-->\n        <!--p:py-->\n        <!--p:pz-->\n        <div data-c=\"counter\" data-b=\"p12\"><button class=\"inc\" data-b=\"p10\">+</button><span class=\"count\" data-b=\"p11\"></span><span class=\"owner\" data-b=\"pw\"></span></div>\n        <button class=\"rm\" data-b=\"p13\">x</button>\n      </li>",
    z3: "<li id=\"empty\">empty</li>",
    t5: "<option data-b=\"p14\"></option>",
    t6: "<span id=\"response\" data-b=\"p15\"></span>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"b":[[0,["",["state","name"],""]]]},
    p1: {"b":[[2,["state","link"],0,"href"],[2,["state","disabled"],0,"aria-disabled"],[2,["state","title"],0,"title"],[2,["state","active"],0,"data-x"]]},
    p2: {"b":[[3,["state","active"],0,"active"],[3,["state","active"],1,"inactive"]]},
    p3: {"b":[[1,["state","active"],0]]},
    p4: {"b":[[1,["state","active"],1]]},
    p5: {"t":1,"s":"t1","p":["state","visible"],"n":0},
    p6: {"t":3,"s":"z1"},
    p7: {"t":2,"s":"t2","v":"item","p":["state","items"],"k":["item","id"]},
    p8: {"t":3,"s":"z3"},
    p9: {"b":[[0,["",["p","name"],""]]]},
    pa: {"w":[["p",["state","person"]]]},
    pb: {"m":["state","name"]},
    pc: {"m":["state","accepted"]},
    pd: {"m":["state","qty"]},
    pe: {"t":2,"s":"t5","v":"c","p":["state","colors"],"k":["c"]},
    pf: {"m":["state","color"]},
    pg: {"m":["state","size"]},
    ph: {"b":[[0,["",["state","qty"],""]]]},
    pi: {"b":[[0,["",["state","accepted"],""]]]},
    pj: {"b":[[0,["",["state","size"],""]]]},
    pk: {"b":[[0,["",["state","color"],""]]]},
    pl: {"e":{"click":[["toggle"],[0]]}},
    pm: {"e":{"click":[["save"],[0]]}},
    pn: {"e":{"click":[["fail"],[0]]}},
    po: {"b":[[1,["state","saving"],0]]},
    pp: {"t":1,"s":"t6","p":["state","response"],"n":0},
    pq: {"b":[[0,["",["state","error"],""]]]},
    pr: {"e":{"click":[["buy"],[0]]}},
    ps: {"b":[[0,["",["state","tick"],""]]]},
    pt: {"e":{"submit":[["submit"],[0]]}},
    pu: {"b":[[0,["",["state","sent"],""]]]},
    pv: {"b":[[0,["",["tag","t"],""]]]},
    pw: {"b":[[0,["",["item","name"],""]]]},
    px: {"t":1,"s":"t3","p":["item","ok"],"n":0},
    py: {"t":2,"s":"t4","v":"tag","p":["item","tags"],"k":["tag","t"]},
    pz: {"t":3,"s":"z2"},
    p10: {"e":{"click":[["inc"],[0]]}},
    p11: {"b":[[0,["",["state","n"],""]]]},
    p12: {"x":1,"c":"counter"},
    p13: {"e":{"click":[["remove"],[["item"]]]}},
    p14: {"b":[[2,["c"],0,"value"],[0,["",["c"],""]]]},
    p15: {"b":[[0,["",["state","response","input","name"],""]]]},
  },
}
export default R
start(R)
