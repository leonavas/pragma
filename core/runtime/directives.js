import { readBy, forgetReads } from './state.js'
import { cssValue, lookup, template as parts } from './binding.js'
import { KEY, LAYOUT_PREFIX, OF } from './contract.js'

const TEXT = 0, SHOW = 1, ATTR = 2, CLASS = 3, ATTR_TEMPLATE = 4, MARKER = 5, CSS_VAR = 6
const IF = 1, FOR = 2, ELSE = 3, ELIF = 4
export const COMPONENT = 1, PAGE = 2

const D = Symbol(), S = Symbol(), L = Symbol(), NONE = { b: [] }
const outer = []
const SELF_DEPTH = 64
let deep = 0
let layoutBodies = null, pageBodies = null
const parsed = new WeakMap(), overlays = new WeakMap()
const NS = { 'http://www.w3.org/2000/svg': 'svg', 'http://www.w3.org/1998/Math/MathML': 'math' }
export const DEV = Boolean(self.__pragmaDev)

export const hooks = { on() {}, enter: (el, s) => s, remove() {} }
export const scopeOf = el => el[S]

const fail = msg => { throw new Error('pragma: ' + msg) }
const path = v => v.split('.')
const cond = v => (v[0] === '!' ? [path(v.slice(1)), true] : [path(v), false])
const JSON_LITERAL = /^(?:true|false|null|-?\d+(?:\.\d+)?|".*")$/s
const NUMBER = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/
const operand = v => (JSON_LITERAL.test(v) ? { v: JSON.parse(v) } : path(v))

const unknown = p => (DEV ? fail(`unknown path "${p}"`) : undefined)
const get = (s, p) => lookup(s, p, unknown)
const template = v => parts(v).map((x, i) => (i % 2 ? path(x) : x))
const str = v => (v == null ? '' : String(v))
const css = (v, name) => cssValue(v, v => (DEV ? fail(`:--${name} got "${v}": a CSS value cannot contain ; { } or <`) : null))

export function useBodies(mod, boot) {
  pageBodies = mod ?? null
  if (boot) layoutBodies = pageBodies
}

const OVERLAY = /<dialog[\s>]|\spopover[\s=>]/

function bodiesHaveOverlay(mod) {
  if (!mod) return false
  let found = overlays.get(mod)
  if (found === undefined) overlays.set(mod, (found = [mod.default, mod.d].some(src => src && Object.values(src).some(html => OVERLAY.test(html)))))
  return found
}

export const hasOverlay = root => !!root.querySelector('dialog,[popover]') || bodiesHaveOverlay(pageBodies) || bodiesHaveOverlay(layoutBodies)

function bodyOf(src, id, ns) {
  let m = parsed.get(src)
  if (!m) parsed.set(src, (m = new Map()))
  let r = m.get(id)
  if (!r) {
    const t = document.createElement('template')
    t.innerHTML = ns ? `<${ns}>${src[id]}</${ns}>` : src[id]
    r = t.content.firstElementChild
    m.set(id, (r = ns ? r.firstElementChild : r))
  }
  return r
}

function source(id, parent) {
  const src = (id.startsWith(LAYOUT_PREFIX) ? layoutBodies : pageBodies)?.default
  return src && Object.hasOwn(src, id) ? bodyOf(src, id, NS[parent.namespaceURI]) : fail(`no template body "${id}": the page's template module is missing or out of date`)
}

function fillSelf(el, id) {
  const src = [pageBodies, layoutBodies].map(m => m?.d).find(d => d && Object.hasOwn(d, id)) ?? fail(`<${el.getAttribute('data-self')}> renders itself but the template module has no definition "${id}"`)
  el.append(...bodyOf(src, id, NS[el.namespaceURI]).cloneNode(true).childNodes)
}

function fill(s, t) {
  let o = t[0]
  for (let i = 1; i < t.length; i += 2) o += str(get(s, t[i])) + t[i + 1]
  return o
}

const EVENT = {}, EL = {}
const LITERAL = { true: true, false: false, null: null }
function call(v) {
  const m = /^\s*([\w$.]+)\s*(?:\(([^]*)\))?\s*$/.exec(v ?? '') ?? fail(`invalid event handler "${v}"`)
  const args = m[2] == null ? [EVENT] : (m[2].match(/'[^']*'|"[^"]*"|[^,\s]+/g) ?? []).map(a =>
    a === 'event' ? EVENT
    : a === 'el' ? EL
    : a[0] === "'" || a[0] === '"' ? { v: a.slice(1, -1) }
    : a in LITERAL ? { v: LITERAL[a] }
    : NUMBER.test(a) && isFinite(a) ? { v: +a }
    : { p: path(a) })
  return { f: m[1], args }
}

export function eventCall(el, type, event) {
  const d = el[D] ?? parse(el), c = ((d.c ??= {})[type] ??= call(el.getAttribute('data-on-' + type))), s = el[S]
  const args = c.args.map(a => (a === EVENT ? event : a === EL ? el : 'v' in a ? a.v : s && get(s, a.p)))
  if (!c.f.includes('.')) return [c.f, args]
  const p = path(c.f), store = s && get(s, p.slice(0, -1)), fn = store?.[p.at(-1)]
  return [c.f, args, typeof fn === 'function' ? fn.bind(store) : undefined]
}

const ANCHOR = /^p:(if|elif|else|for|empty)(\d*) (\S+)(?: (.*))?$/
const decode = v => (v.includes('%') ? decodeURIComponent(v) : v)

function anchor(n) {
  const m = ANCHOR.exec(n.data)
  if (!m) return (n[D] = NONE)
  const [, kind, slot, s, rest] = m, a = rest ? rest.split(' ').map(decode) : [], d = { s, u: +slot }
  if (kind === 'if' || kind === 'elif') [d.p, d.n] = cond(a[0]), (d.t = kind === 'if' ? IF : ELIF)
  else if (kind === 'for') Object.assign(d, { t: FOR, v: a[0], p: path(a[1]), k: path(a[2]) })
  else d.t = ELSE
  if (d.t === IF || d.t === FOR) d.id = s
  return (n[D] = d)
}

function parse(el) {
  const d = { b: [] }
  for (const { name, value } of el.attributes) {
    if (!name.startsWith('data-')) continue
    const n = name.slice(5), i = n.indexOf('-'), a = n.slice(i + 1)
    switch (i < 0 ? n : n.slice(0, i + 1)) {
      case 'text': d.b.push([TEXT, template(value)]); break
      case 'show': d.b.push([SHOW, ...cond(value)]); break
      case 'attr-': d.b.push(value.includes('{') ? [ATTR_TEMPLATE, template(value), 0, a] : [ATTR, path(value), 0, a]); break
      case 'var-': d.b.push([CSS_VAR, path(value), 0, a]); break
      case 'class-': d.b.push([CLASS, ...cond(value), a]); break
      case 'with-': (d.w ??= []).push([a, operand(value)]); break
      case 'on-': hooks.on(a); d.e = 1; break
      case 'model':
        d.m = path(value)
        if (DEV && d.m[0] !== 'state' && d.m[0] !== 'shared') fail(`data-model only accepts state or shared: "${value}"`)
        hooks.on('input'); d.e = 1; break
      case 'c': d.x = COMPONENT; break
      case 'self': d.self = 1; break
      case 'page': d.x = PAGE; break
      case 'slot': d.u = +value
    }
  }
  if (d.self && !el.firstChild) fillSelf(el, el.getAttribute('data-c'))
  for (let n = el.firstChild, m; n; n = n.nextSibling) {
    if (n.nodeType !== 8 || !(m = /^(\d*)\{(.*)\}$/.exec(n.data))) continue
    let t = n.nextSibling
    if (t?.nodeType !== 3) n.after((t = document.createTextNode('')))
    d.b.push([MARKER, path(m[2]), t, +m[1]])
  }
  return (el[D] = d.b.length || d.w || d.e || d.x || d.u || d.self ? d : NONE)
}

function bind(el, [t, p, n, a], s) {
  let v
  if (t === TEXT) {
    if (el.textContent !== (v = fill(s, p))) el.textContent = v
  } else if (t === ATTR_TEMPLATE) {
    if (el.getAttribute(a) !== (v = fill(s, p))) el.setAttribute(a, v)
  } else if (t === MARKER) {
    if (n.data !== (v = str(get(s, p)))) n.data = v
  } else if (t === CSS_VAR) {
    v = css(get(s, p), a)
    if (el.style.getPropertyValue('--' + a) !== (v ?? '')) v == null ? el.style.removeProperty('--' + a) : el.style.setProperty('--' + a, v)
  } else if (t === ATTR) {
    v = get(s, p)
    v = v === false || v == null ? null : v === true ? '' : String(v)
    if (el.getAttribute(a) !== v) v == null ? el.removeAttribute(a) : el.setAttribute(a, v)
    if ((a === 'checked' || a === 'selected') && el[a] !== (v != null)) el[a] = v != null
  } else {
    v = !get(s, p) === n
    if (t === SHOW) { if (el.hasAttribute('hidden') === v) el.toggleAttribute('hidden', !v) }
    else if (el.classList.contains(a) !== v) el.classList.toggle(a, v)
  }
}

function model(el, s, p) {
  const v = get(s, p), t = el.type
  if (t === 'checkbox' || t === 'radio') {
    const c = t === 'radio' ? v === el.value : !!v
    if (el.checked !== c) el.checked = c
    return
  }
  const same = t === 'number' || t === 'range' ? (v == null ? el.value === '' : el.valueAsNumber === v) : el.value === String(v ?? '')
  if (!same) el.value = v ?? ''
}

export function input(e) {
  const el = e.target, p = el[D]?.m
  if (!p || (el.type === 'radio' && !el.checked)) return false
  const t = el.type
  const v = t === 'checkbox' ? el.checked : t === 'number' || t === 'range' ? (el.value === '' ? null : el.valueAsNumber) : el.value
  get(el[S], p.slice(0, -1))[p.at(-1)] = v
  return true
}

const lexical = (s, d) => (d.u && outer.at(-d.u)) || s

function visit(el, s, d = el[D] ?? parse(el)) {
  s = lexical(s, d)
  let w = null
  if (d.w) {
    w = {}
    for (const [a, p] of d.w) w[a] = 'v' in p ? p.v : get(s, p)
    s = Object.assign({ __proto__: s }, w)
  }
  if (d.self && ++deep > SELF_DEPTH) fail(`<${el.getAttribute('data-self')}> is nested more than ${SELF_DEPTH} levels inside itself: stop the recursion on data (a tree without cycles, <if>/<for> around the self call)`)
  if (d.x === COMPONENT) outer.push(s)
  if (d.x) s = hooks.enter(el, s, d.x, w)
  for (const b of d.b) {
    const r = [el, b, b[0] === MARKER && b[3] ? (outer.at(-b[3]) ?? s) : s]
    readBy(r)
    bind(...r)
  }
  readBy(0)
  if (d.e) el[S] = s
  if (el.firstChild) walk(el, s)
  if (d.m) model(el, s, d.m)
  if (d.x === COMPONENT) outer.pop()
  if (d.self) deep--
}

function walk(parent, s) {
  let last, id
  for (let n = parent.firstChild; n; n = n.nextSibling) {
    if (n.nodeType === 1) {
      if (!n[L]) visit(n, s)
    } else if (n.nodeType === 8) {
      const d = n[D] ?? anchor(n)
      if (d.t) [n, last, id] = block(n, d, lexical(s, d), last, d.id ?? id)
    }
  }
}

let leaving = [], entering = [], focusing = null
const reduced = matchMedia('(prefers-reduced-motion: reduce)')

function clone(t, d, id) {
  const n = source(d.s, t.parentNode).cloneNode(true)
  n.setAttribute(OF, id)
  if (d.rendered && !reduced.matches) {
    n.setAttribute('data-entering', '')
    entering.push(n)
  }
  return n
}

function drop(n) {
  if (n[L]) return
  n[L] = {}
  leaving.push(n)
}

function stay(n) {
  n[L] = null
  n.removeAttribute('data-leaving')
  n.inert = false
}

function gone(n, token) {
  if (n[L] !== token) return
  n[L] = null
  hooks.remove(n)
  n.remove()
}

function timing(n) {
  const finished = []
  let ms = 0
  for (const a of n.getAnimations({ subtree: true })) {
    const end = a.effect?.getComputedTiming().endTime
    if (!(end < Infinity)) continue
    finished.push(a.finished)
    ms = Math.max(ms, end - (a.currentTime ?? 0))
  }
  return [finished, ms]
}

const still = [[], 0]

export const afterMotion = (n, done) => after(reduced.matches ? still : timing(n), done)

function after([finished, ms], done) {
  if (!finished.length) return done()
  Promise.race([Promise.allSettled(finished), new Promise(r => setTimeout(r, ms + 50))]).then(done)
}

function entered(into) {
  const waits = into.filter(n => n.hasAttribute('data-entering')).map(n => [timing(n), () => n.removeAttribute('data-entering')])
  for (const [m, done] of waits) after(m, done)
}

function settle() {
  if (!leaving.length && !entering.length) return
  const out = leaving.filter(n => n[L]), into = entering, motion = !reduced.matches
  leaving = []
  entering = []
  if (motion) for (const n of out) {
    n.removeAttribute('data-entering')
    n.setAttribute('data-leaving', '')
    n.inert = true
  }
  const waits = out.map(n => [motion ? timing(n) : still, gone.bind(null, n, n[L])])
  for (const [m, done] of waits) after(m, done)
  if (into.length) requestAnimationFrame(() => setTimeout(entered, 0, into))
}

const sibling = n => {
  do n = n.nextSibling
  while (n?.nodeType === 3)
  return n
}
const of = (n, id) => n?.nodeType === 1 && n.getAttribute(OF) === id
const next = n => {
  do n = sibling(n)
  while (n?.[L])
  return n
}

function block(t, d, s, last, id) {
  const r = d.t === FOR ? list(t, d, s) : branch(t, d, s, last, id)
  d.rendered = 1
  return r
}

function branch(t, d, s, last, id) {
  const chained = (d.t === ELSE || d.t === ELIF) && last
  const on = !chained && (d.t === ELSE || !get(s, d.p) === d.n)
  let n = sibling(t)
  if (!of(n, id)) n = null
  if (!on) {
    if (n) drop(n)
    return [t, chained, id]
  }
  if (n) {
    if (n[L]) stay(n)
    visit(n, s)
  } else {
    t.after((n = clone(t, d, id)))
    visit(n, s)
    focusing = (n.matches('[autofocus]') ? n : n.querySelector('[autofocus]')) ?? focusing
  }
  return [n, true, id]
}

function list(t, d, s) {
  const id = d.id, old = new Map(), scopes = [], keys = []
  for (let n = sibling(t); of(n, id); n = sibling(n)) old.set(n.getAttribute(KEY), n)
  for (const it of get(s, d.p) ?? []) {
    const c = { __proto__: s, [d.v]: it }
    scopes.push(c)
    keys.push(String(get(c, d.k)))
  }
  const keep = new Set(keys)
  if (DEV && keep.size < keys.length) fail(`duplicate key in <for ${d.v} in ${d.p.join('.')}>`)
  for (const [k, n] of old) if (!keep.has(k)) drop(n)
  let ref = t
  for (let i = 0; i < keys.length; i++) {
    let n = old.get(keys[i])
    if (!n) {
      n = clone(t, d, id)
      n.setAttribute(KEY, keys[i])
      ref.after(n)
    } else {
      if (n[L]) stay(n)
      if (next(ref) !== n) ref.after(n)
    }
    visit(n, scopes[i])
    ref = n
  }
  return [ref, keys.length > 0, id]
}

export function render(el, s) {
  outer.length = 0
  deep = 0
  forgetReads()
  readBy(0)
  focusing = null
  try { visit(el, s) } finally { readBy(null) }
  settle()
  const target = focusing
  focusing = null
  target?.focus()
}

export function patch(groups) {
  try {
    for (const g of groups) for (const r of g) {
      readBy(r)
      bind(...r)
    }
  } finally { readBy(null) }
}
