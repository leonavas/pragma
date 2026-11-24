import { readBy, forgetReads } from './state.js'

// One walker over the HTML the build wrote. An element with `data-b="<id>"` and a comment
// `<!--p:<id>-->` (block anchor) take their descriptor from the route tables (R.b); the
// descriptors are immutable and shared, so all per-node state lives on the node under symbols.
// Blocks, model, motion, self-recursion and CSS variables plug in through H and B (per-use modules).

export const DEV = Boolean(self.__pragmaDev)
export const D = Symbol(), S = Symbol(), L = Symbol(), RD = Symbol(), PART = Symbol()
const REC = Symbol(), MREC = Symbol(), OWN = Symbol(), MK = Symbol(), EN = Symbol(), OPENED = Symbol()
const NONE = {}

// Scope of each open component; slot content (`u` = n) uses the one n levels up.
export const outer = []
export const ctx = { deep: 0, focus: null, end: Infinity }
export const K = { overlay: null }
export const B = {}

export function waitsForCss(id) {
  if (K.cssBlocks?.has(id)) return (K.cssMiss = true)
  return false
}

export const fail = m => { throw new Error('pragma: ' + m) }
const missing = (what, file) => (DEV ? fail(`${what} needs ${file}.js (build bug)`) : undefined)
const skip = (what, file) => (t, d, s, last, id) => (missing(what, file), [t, false, id])

export const H = {
  on() {}, enter: (el, s) => s, remove() {}, reveal() {},
  branch: skip('<if>', 'branch'), list: skip('<for>', 'list'),
  made() {}, drop: n => { H.remove(n); n.remove() }, stay() {}, done() {},
  self: () => missing('data-self', 'self'), depth() {},
  input() {}, whole() {}, flushed() {},
}

// Tables: ids that start with `l` (layout region) come from the route the document booted on,
// the others from the current route.
const T = [null, null]
export function useTables(R, boot) {
  T[1] = R
  if (boot) T[0] = R
}
function from(k, id) {
  const t = T[id[0] === 'l' ? 0 : 1]?.[k]
  return t && Object.hasOwn(t, id) ? t[id] : undefined
}
const outdated = (what, id) => fail(`no ${what} "${id}": the route entry is missing or out of date`)
export const desc = id => from('b', id) ?? (DEV ? outdated('descriptor', id) : NONE)

const NS = { 'http://www.w3.org/2000/svg': 'svg', 'http://www.w3.org/1998/Math/MathML': 'math' }
const parsed = new Map()
function parse(html, parent) {
  const ns = NS[parent.namespaceURI], key = ns + html
  let r = parsed.get(key)
  if (!r) {
    const t = document.createElement('template')
    t.innerHTML = ns ? `<${ns}>${html}</${ns}>` : html
    r = t.content.firstElementChild
    parsed.set(key, (r = ns ? r.firstElementChild : r))
  }
  return r
}
export const body = (id, parent) => parse(from('t', id) ?? outdated('template body', id), parent)
// Self-call bodies: current route first, then the boot route.
export const def = (id, el) => parse(T[1]?.d?.[id] ?? T[0]?.d?.[id] ?? outdated('definition', id), el)

export function get(s, p) {
  for (const k of p) {
    if (s == null) return
    if (!(k in Object(s))) return DEV && p[0] !== 'query' ? fail(`unknown path "${p.join('.')}"`) : undefined
    const o = s
    s = s[k]
    if (typeof s === 'function') s = s.call(o)
  }
  return s
}
const str = v => (v == null ? '' : String(v))
export function fill(s, t) {
  let o = t[0]
  for (let i = 1; i < t.length; i += 2) o += str(get(s, t[i])) + t[i + 1]
  return o
}
const set = (el, a, v) => el.getAttribute(a) !== v && (v == null ? el.removeAttribute(a) : el.setAttribute(a, v))

// Text nodes after the element's direct <!--{--> markers, in order (MARK binding `n` = index).
function marks(el) {
  let m = el[MK]
  if (!m) {
    m = el[MK] = []
    for (let n = el.firstChild; n; n = n.nextSibling) {
      if (n.nodeType !== 8 || n.data !== '{') continue
      let t = n.nextSibling
      if (t?.nodeType !== 3) n.after((t = document.createTextNode('')))
      m.push(t)
    }
  }
  return m
}

// [kind, a, n, name]: 0 text, 1 show, 2 attr, 3 class, 4 attr template, 5 marker; others in B.
export function bind(el, b, s) {
  const [t, p, n, a] = b
  let v
  switch (t) {
    case 0:
      v = fill(s, p)
      if (el.textContent !== v) el.textContent = v
      break
    case 4:
      set(el, a, fill(s, p))
      break
    case 5: {
      const m = marks(el)[n]
      v = str(get(s, p))
      if (m && m.data !== v) m.data = v
      break
    }
    case 2:
      v = get(s, p)
      set(el, a, v === false || v == null ? null : v === true ? '' : String(v))
      // Once the user touches it, the attribute no longer drives the state: set the property.
      if ((a === 'checked' || a === 'selected') && el[a] !== (v != null && v !== false)) el[a] = v != null && v !== false
      break
    case 1:
    case 3:
      v = !get(s, p) === !!n
      if (t === 1) el.hasAttribute('hidden') === v && el.toggleAttribute('hidden', !v)
      else if (el.classList.contains(a) !== v) el.classList.toggle(a, v)
      break
    default:
      B[t] ? B[t](el, b, s) : missing(`binding kind ${t}`, t === 6 ? 'vars' : 'model')
  }
}

// A closed <dialog> or [popover] binds only its own attributes: its content renders on the first
// open and stays after that.
const closed = el => !el[OPENED] && (el.localName === 'dialog' ? !el.open : el.hasAttribute('popover') && !el.matches(':popover-open'))
export function reveal(el) {
  if (el[OPENED]) return
  el[OPENED] = 1
  H.reveal()
}

// Owner record: what a re-visit of `el` needs (scope before lexical/with/component, outer stack,
// self depth). Structural reads (with, component, anchors among the children) are tracked under it.
function own(el, s, n, dp) {
  const o = (el[OWN] ??= { el })
  o.s = s
  o.o = outer.slice(0, n)
  o.dp = dp
  return o
}

export function visit(el, s0) {
  let d = el[D]
  if (!d) {
    const id = el.getAttribute('data-b')
    d = el[D] = id ? desc(id) : NONE
  }
  const n0 = outer.length, dp = ctx.deep
  let s = (d.u && outer.at(-d.u)) || s0, ow, w
  if (d.e) for (const t in d.e) H.on(t)
  if (d.m) H.on('input')
  // A JS-only control leaves the build disabled; it works from here (once: a later :disabled wins).
  if (d.d && !el[EN] && (!d.o || K.overlay)) {
    el[EN] = 1
    el.removeAttribute('disabled')
  }
  if (d.w) {
    readBy((ow ??= own(el, s0, n0, dp)))
    w = {}
    for (const [a, p] of d.w) w[a] = Array.isArray(p) ? get(s, p) : p.v
    s = Object.assign({ __proto__: s }, w)
  }
  if (d.self) {
    H.depth(1, el)
    if (!el.firstChild) H.self(el, d)
  }
  if (d.x === 1) outer.push(s)
  if (d.x) {
    readBy((ow ??= own(el, s0, n0, dp)))
    s = H.enter(el, s, d, w)
  }
  if (d.b) {
    const rs = (el[REC] ??= [])
    for (let i = 0; i < d.b.length; i++) {
      const b = d.b[i], r = (rs[i] ??= [el, b, null])
      r[2] = b[0] === 5 && b[3] ? outer.at(-b[3]) ?? s : s
      readBy(r)
      bind(el, b, r[2])
    }
  }
  readBy(null)
  if (d.e || d.m) el[S] = s
  if (!closed(el)) {
    let last, id
    for (let n = el.firstChild; n; n = n.nextSibling) {
      if (n.nodeType === 1) n[L] || visit(n, s)
      else if (n.nodeType === 8) {
        const a = n[D] ?? (n[D] = n.data.startsWith('p:') ? desc(n.data.slice(2)) : NONE)
        if (!a.t) continue
        readBy((ow ??= own(el, s0, n0, dp)))
        ;[n, last, id] = (a.t === 2 ? H.list : H.branch)(n, a, (a.u && outer.at(-a.u)) || s, last, a.t < 3 ? a.s : id)
        readBy(null)
      }
    }
  }
  if (d.m) {
    const r = (el[MREC] ??= [el, [7, d.m], null])
    r[2] = s
    readBy(r)
    bind(el, r[1], s)
    readBy(null)
  }
  if (d.x === 1) outer.length = n0
  if (d.self) H.depth(-1, el)
}

export const sib = n => {
  do n = n.nextSibling
  while (n?.nodeType === 3)
  return n
}
export const of = (n, id) => n?.nodeType === 1 && n.getAttribute('data-of') === id

export function clone(t, d, id) {
  const n = body(d.s, t.parentNode).cloneNode(true)
  n.setAttribute('data-of', id)
  H.made(n, t[RD] && !t[PART])
  return n
}

export function scopes(d, s) {
  const sc = [], keys = []
  for (const it of get(s, d.p) ?? []) {
    const c = { __proto__: s, [d.v]: it }
    sc.push(c)
    keys.push(String(get(c, d.k)))
  }
  return [sc, keys]
}

function settle() {
  H.done()
  const f = ctx.focus
  ctx.focus = null
  f?.focus()
}

export function render(el, s) {
  outer.length = 0
  ctx.deep = 0
  ctx.focus = null
  forgetReads()
  H.whole()
  try { visit(el, s) } finally { readBy(null) }
  settle()
}

// After a change: re-visit the owners (outermost first, document order), then patch the bindings
// that are not inside a re-visited owner.
const before = (a, b) => (a.el.compareDocumentPosition(b.el) & 2 ? 1 : -1)
export function update(rs) {
  const kept = []
  for (const o of rs.filter(r => r.el?.isConnected).sort(before)) kept.some(k => k.el.contains(o.el)) || kept.push(o)
  ctx.focus = null
  try {
    for (const o of kept) {
      outer.length = 0
      outer.push(...o.o)
      ctx.deep = o.dp
      visit(o.el, o.s)
    }
    for (const r of rs) if (!r.el && r[0].isConnected && !kept.some(k => k.el.contains(r[0]))) readBy(r), bind(...r)
  } finally { readBy(null) }
  settle()
}
