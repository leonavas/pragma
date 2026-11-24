import { H, K, L, RD, PART, DEV, ctx, outer, fail, visit, clone, of, sib, scopes } from './dom.js'

// Keyed <for>. Large lists render in slices: once a flush has run its budget (ctx.end), an outer
// <for> (not one inside another <for> item) creates at least one more item and leaves the rest
// for the next task. In a reverse-column scroller (a chat that opens at its end) the last items
// come first. Jobs are keyed by anchor: a re-visit of the list replaces its job, a whole render
// drops all of them.
const END = Symbol()
const jobs = new Map()
let lists = 0
const now = () => performance.now()

function scroller(el) {
  for (; el && el !== document.body; el = el.parentElement) {
    const cs = getComputedStyle(el)
    if (cs.overflowY === 'auto' || cs.overflowY === 'scroll') return cs.flexDirection === 'column-reverse'
  }
  return false
}

const next = n => {
  do n = sib(n)
  while (n?.[L])
  return n
}

function list(t, d, s, last, id) {
  const top = lists++ === 0
  try { return items(t, d, s, top, id) } finally { lists-- }
}

function items(t, d, s, top, id) {
  const old = new Map(), [sc, keys] = scopes(d, s), keep = new Set(keys)
  for (let n = sib(t); of(n, id); n = sib(n)) old.set(n.getAttribute('data-key'), n)
  if (DEV && keep.size < keys.length) fail(`duplicate key in <for ${d.v} in ${d.p.join('.')}>`)
  for (const [k, n] of old) if (!keep.has(k)) H.drop(n)
  const fresh = new Set(), skipped = []
  let made = 0, rest = false
  const create = i => {
    const n = clone(t, d, id)
    n.setAttribute('data-key', keys[i])
    made++
    return n
  }
  const late = () => top && made > 0 && now() > ctx.end
  const end = top && old.size < keys.length && (t[END] ??= scroller(t.parentNode))
  if (end) {
    for (let i = keys.length - 1; i >= 0; i--) {
      if (old.has(keys[i])) continue
      if (late()) { rest = true; break }
      const n = create(i)
      t.after(n)
      visit(n, sc[i])
      old.set(keys[i], n)
      fresh.add(n)
    }
  }
  let ref = t
  for (let i = 0; i < keys.length; i++) {
    let n = old.get(keys[i])
    if (!n) {
      if (rest || late()) { rest = true; skipped.push(i); continue }
      n = create(i)
      ref.after(n)
      old.set(keys[i], n)
      fresh.add(n)
    } else {
      if (n[L]) H.stay(n)
      if (next(ref) !== n) ref.after(n)
      if (fresh.has(n)) { ref = n; continue }
    }
    visit(n, sc[i])
    ref = n
  }
  t[RD] = 1
  t[PART] = rest
  if (rest) jobs.set(t, { t, d, id, keys, sc, nodes: old, todo: end ? skipped.reverse() : skipped, end, outer: outer.slice(), deep: ctx.deep })
  else jobs.delete(t)
  return [ref, keys.length > 0, id]
}

// Creates the items a slice left, from where it stopped, each next to a neighbour already on the page.
function more(job) {
  const { t, d, id, keys, sc, nodes, todo, end } = job
  outer.length = 0
  outer.push(...job.outer)
  ctx.deep = job.deep
  let made = 0
  lists++
  try {
    while (todo.length) {
      if (made && now() > ctx.end) return false
      const i = todo.shift(), n = clone(t, d, id)
      n.setAttribute('data-key', keys[i])
      const near = nodes.get(keys[end ? i + 1 : i - 1])
      if (end) near ? near.before(n) : t.after(n)
      else (near ?? t).after(n)
      visit(n, sc[i])
      nodes.set(keys[i], n)
      made++
    }
  } finally { lists-- }
  t[PART] = false
  return true
}

function resume() {
  for (const [t, job] of jobs) {
    if (t.isConnected && !more(job)) break
    jobs.delete(t)
  }
  H.done()
}

export default function install() {
  H.list = list
  H.whole = () => jobs.clear()
  K.slices = { resume, pending: () => jobs.size > 0 }
}
