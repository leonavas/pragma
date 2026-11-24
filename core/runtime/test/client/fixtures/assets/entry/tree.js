import { start } from '/core/page.js'
import branch from '/core/branch.js'
import list from '/core/list.js'
import model from '/core/model.js'
import self from '/core/self.js'
import vars from '/core/vars.js'
import persist from '/core/persist.js'
import net from '/core/net.js'
import page from '/assets/tree.js'
import c0 from '/assets/node.js'

const R = {
  pattern: "/tree",
  use: [branch, list, model, self, vars, persist, net],
  page,
  comps: { "node": c0 },
  stores: {  },
  t: {
    t2: "<circle r=\"1\" data-b=\"p8\"></circle>",
    t1: "<li data-b=\"p9\"></li>",
    t9: "<li data-c=\"node\" class=\"node\" data-b=\"p6\"></li>",
  },
  d: {
    node: "<li data-c=\"node\" class=\"node\" data-b=\"pa\"><span data-b=\"p4\"></span><ul><!--p:p5--></ul></li>",
  },
  b: {
    l0: {"x":2},
    p0: {"b":[[0,["",["state","count"],""]]]},
    p1: {"b":[[6,["state","left"],0,"left"],[6,["state","tone"],0,"tone"]]},
    p2: {"t":2,"s":"t1","v":"row","p":["state","rows"],"k":["row","id"]},
    p3: {"t":2,"s":"t2","v":"row","p":["state","rows"],"k":["row","id"]},
    p4: {"b":[[0,["",["state","node","name"],""]]]},
    p5: {"t":2,"s":"t9","v":"child","p":["state","node","children"],"k":["child","id"]},
    p6: {"x":1,"c":"node","self":"c-node","b":[[6,["child","depth"],0,"depth"]],"w":[["node",["child"]]]},
    p7: {"x":1,"c":"node","w":[["node",["state","root"]]]},
    p8: {"b":[[2,["row","id"],0,"cx"]]},
    p9: {"b":[[6,["row","x"],0,"x"],[0,["",["row","id"],""]]]},
    pa: {"x":1,"c":"node"},
  },
}
export default R
start(R)
