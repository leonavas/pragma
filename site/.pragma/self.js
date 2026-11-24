import { H, D, ctx, def, fail, waitsForCss } from './dom.js'

// A component inside itself arrives with an empty root; its body is in the route's `d` table.
const SELF_DEPTH = 64

export default function install() {
  H.self = (el, d) => {
    if (!waitsForCss('@' + d.c)) el.append(...def(d.c, el).cloneNode(true).childNodes)
  }
  H.depth = (dir, el) => {
    if ((ctx.deep += dir) > SELF_DEPTH) fail(`<${el[D].self}> is nested more than ${SELF_DEPTH} levels inside itself: stop the recursion on data (a tree without cycles, <if>/<for> around the self call)`)
  }
}
