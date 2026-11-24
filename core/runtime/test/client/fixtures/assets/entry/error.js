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
  pattern: "/error",
  loader: 1,
  use: [branch, list, model, self, vars, persist, net],
  comps: {  },
  stores: { "cart": [s0, s0p] },
  t: {
    p1: "<div class=\"skeleton\" id=\"sk\">loading</div>",
    p2: "<p id=\"error\" data-b=\"p2\"></p>",
    z1: "<h1 id=\"name\" data-b=\"p3\"></h1>",
    z2: "<div id=\"ready\">\n      <!--p:p4-->\n      <!--p:p5-->\n    </div>",
  },
  b: {
    l0: {"b":[[0,["",["shared","cart","items","length"],""]]]},
    l1: {"x":2},
    p0: {"t":1,"s":"p1","p":["page","loading"],"n":0},
    p1: {"t":3,"s":"z2"},
    p2: {"b":[[0,["",["page","error","message"],""]]]},
    p3: {"b":[[0,["",["data","product","name"],""]]]},
    p4: {"t":1,"s":"p2","p":["page","error"],"n":0},
    p5: {"t":3,"s":"z1"},
  },
}
export default R
start(R)
