import { start } from "/assets/core/page.js"
import branch from "/assets/core/branch.js"
import list from "/assets/core/list.js"
import page from "/assets/routes/index.js"
import c0 from "/assets/lib/components/card/card.js"
import c1 from "/assets/lib/components/counter/counter.js"
import s0 from "/assets/lib/state/cart.js"

const R = {
  pattern: "/",
  overlays: 1,
  use: [branch, list],
  page,
  comps: {
    card: c0,
    counter: c1,
  },
  stores: {
    cart: [s0],
  },
  t: {
    t0: "<span class=\"badge\">★</span>",
    t1: "<span>-</span>",
    t2: "<div data-c=\"card\" class=\"card p-4\" data-b=\"pa\">\n  <h2 class=\"text-lg\" data-b=\"p2\"></h2>\n  <!--p:p3-->\n  <!--p:p4-->\n  <b>promo</b>\n  <footer><button class=\"btn\" disabled data-b=\"p5\">Buy</button><small>no actions</small></footer>\n</div>",
    t3: "<i>normal</i>",
    t4: "<li>\n        <span data-b=\"pb\"></span>\n        <!--p:pc-->\n        <!--p:pd-->\n      </li>",
    t5: "<li>Nothing</li>",
  },
  b: {
    l0: {"x":1,"c":"counter","e":{"click":[["add"],[0]]},"d":1},
    l1: {"b":[[0,["",["state","count"],""]]]},
    l2: {"b":[[0,["",["shared","cart","total"],""]]]},
    l3: {"x":2},
    l4: {"b":[[0,["",["query","ref"],""]]]},
    p0: {"o":1,"e":{"click":[["toggle"],[0]]},"d":1},
    p1: {"x":1,"c":"card","b":[[3,["state","featured"],0,"featured"]],"p":{"heading":"Featured","featured":"true"}},
    p2: {"b":[[0,["",["state","heading"],""]]]},
    p3: {"t":1,"s":"t0","p":["state","featured"],"n":0},
    p4: {"t":3,"s":"t1"},
    p5: {"e":{"click":[["buy"],[0]]},"d":1},
    p6: {"u":1,"b":[[0,["",["state","open"],""]]],"e":{"click":[["toggle"],[0]]},"d":1},
    p7: {"b":[[1,["state","open"],0]]},
    p8: {"t":2,"s":"t4","v":"item","p":["state","items"],"k":["item","id"]},
    p9: {"t":3,"s":"t5"},
    pa: {"x":1,"c":"card","b":[[3,["state","featured"],0,"featured"]],"p":{"heading":"Promo"}},
    pb: {"b":[[0,["",["item","name"],""]]]},
    pc: {"t":1,"s":"t2","p":["item","promo"],"n":0},
    pd: {"t":3,"s":"t3"},
  },
}
export default R
start(R)
