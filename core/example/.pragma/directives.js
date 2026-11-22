import { readBy, forgetReads } from './state.js'

const D = Symbol(), S = Symbol(), L = Symbol(), NONE = { b: [] }
// Outer scope of each open component; slot content (data-slot="n") uses the one n levels up.
const outer = []
let auto = 0
// Open `data-self` roots (a component inside itself); more than SELF_DEPTH means the data never ends.
export const SELF_DEPTH = 64
let deep = 0
const defs = new Map(), tpls = new Map()

export const hooks = { on() {}, enter: (el, s) => s, remove() {} }
export const scopeOf = el => el[S]

const fail = msg => { throw new Error('pragma: ' + msg) }
// Inside <svg> a <template> is a plain SVG element without .content: its body is its own children.
const body = t => t.content ?? t
const path = v => v.split('.')
const cond = v => (v[0] === '!' ? [path(v.slice(1)), true] : [path(v), false])
const JSON_LITERAL = /^(?:true|false|null|-?\d+(?:\.\d+)?|".*")$/s
const operand = v => (JSON_LITERAL.test(v) ? { v: JSON.parse(v) } : path(v))

// A function met on a path is a derived value: called with no args.
export function get(s, p) {
  let v = s
  for (const k of p) {
    const o = v
    if (o == null || !(k in Object(o))) {
      if (self.__pragmaDev && p[0] !== 'query') fail(`unknown path "${p.join('.')}"`)
      return
    }
    v = o[k]
    if (typeof v === 'function') v = v.call(o)
  }
  return v
}

// Template "a {path} b": [literal, path, literal, …]; `\{` is a literal brace.
const TPL = /\\\{|\{([^{}]*)\}/g
export function template(v) {
  const t = ['']
  let at = 0
  for (const m of v.matchAll(TPL)) {
    t[t.length - 1] += v.slice(at, m.index)
    if (m[1] == null) t[t.length - 1] += '{'
    else t.push(path(m[1]), '')
    at = m.index + m[0].length
  }
  t[t.length - 1] += v.slice(at)
  return t
}

const str = v => (v == null ? '' : String(v))

// `:--name`: false, null, undefined and '' remove the property; a value that could add declarations is refused.
function cssValue(v, name) {
  if (v == null || v === false || v === '') return null
  v = String(v)
  if (!/[;{}<]/.test(v)) return v
  if (self.__pragmaDev) fail(`:--${name} got "${v}": a CSS value cannot contain ; { } or <`)
  return null
}

function source(cache, attr, id, missing) {
  let t = cache.get(id)
  if (!t?.isConnected) cache.set(id, (t = document.querySelector(`template[${attr}="${CSS.escape(id)}"]`) ?? fail(missing())))
  return body(t).firstElementChild
}

// A component inside itself: its root arrives empty and gets the body from the page's <template data-def>.
function fillSelf(el, id) {
  el.append(...source(defs, 'data-def', id, () => `<${el.getAttribute('data-self')}> renders itself but the page has no <template data-def="${id}">`).cloneNode(true).childNodes)
}

const root = (t, d) => body(t).firstElementChild ?? (d.s ? source(tpls, 'data-tpl', d.s, () => `the page has no <template data-tpl="${d.s}">`) : null)

function fill(s, t) {
  let o = t[0]
  for (let i = 1; i < t.length; i += 2) o += str(get(s, t[i])) + t[i + 1]
  return o
}

// `fn` (gets the event), `fn(arg, …)` or `shared.store.fn(…)`; arg: path, 'text', "text", number,
// true, false, null, event or el (the element with the directive).
const EVENT = {}, EL = {}
const LITERAL = { true: true, false: false, null: null }
function call(v) {
  const m = /^\s*([\w$.]+)\s*(?:\(([^]*)\))?\s*$/.exec(v ?? '') ?? fail(`invalid event handler "${v}"`)
  const args = m[2] == null ? [EVENT] : (m[2].match(/'[^']*'|"[^"]*"|[^,\s]+/g) ?? []).map(a =>
    a === 'event' ? EVENT
    : a === 'el' ? EL
    : a[0] === "'" || a[0] === '"' ? { v: a.slice(1, -1) }
    : a in LITERAL ? { v: LITERAL[a] }
    : /^-?\d+(\.\d+)?$/.test(a) ? { v: +a }
    : { p: path(a) })
  return { f: m[1], args }
}

// Handler written in `data-on-<type>`: [function name, arguments resolved against the element's
// scope, the function itself for a store handler (`shared.cart.add`)].
export function eventCall(el, type, event) {
  const d = el[D] ?? parse(el), c = ((d.c ??= {})[type] ??= call(el.getAttribute('data-on-' + type))), s = el[S]
  const args = c.args.map(a => (a === EVENT ? event : a === EL ? el : 'v' in a ? a.v : s && get(s, a.p)))
  if (!c.f.includes('.')) return [c.f, args]
  const p = path(c.f), store = s && get(s, p.slice(0, -1)), fn = store?.[p.at(-1)]
  return [c.f, args, typeof fn === 'function' ? fn.bind(store) : undefined]
}

function parse(el) {
  if (el.localName === 'template') {
    const d = {}
    let v
    if ((v = el.getAttribute('data-if')) != null) [d.p, d.n] = cond(v), (d.t = 1)
    else if ((v = el.getAttribute('data-for')) != null) {
      const [x, , p] = v.trim().split(/\s+/)
      Object.assign(d, { t: 2, v: x, p: path(p), k: path(el.getAttribute('data-key') ?? fail(`<for> without key: "${v}"`)) })
    } else if (el.hasAttribute('data-else') || el.hasAttribute('data-empty')) d.t = 3
    else if ((v = el.getAttribute('data-else-if')) != null) [d.p, d.n] = cond(v), (d.t = 4)
    d.u = +el.getAttribute('data-slot')
    d.s = el.getAttribute('data-tid')
    if (d.t < 3) d.id = d.s ?? (el.setAttribute('data-tid', (v = 't' + auto++)), v)
    return (el[D] = d.t ? d : NONE)
  }
  const d = { b: [] }
  for (const { name, value } of el.attributes) {
    if (!name.startsWith('data-')) continue
    const n = name.slice(5), i = n.indexOf('-'), a = n.slice(i + 1)
    switch (i < 0 ? n : n.slice(0, i + 1)) {
      case 'text': d.b.push([0, template(value)]); break
      case 'show': d.b.push([1, ...cond(value)]); break
      case 'attr-': d.b.push(value.includes('{') ? [4, template(value), 0, a] : [2, path(value), 0, a]); break
      case 'var-': d.b.push([6, path(value), 0, a]); break
      case 'class-': d.b.push([3, ...cond(value), a]); break
      case 'with-': (d.w ??= []).push([a, operand(value)]); break
      case 'on-': hooks.on(a); d.e = 1; break
      case 'model':
        d.m = path(value)
        if (self.__pragmaDev && d.m[0] !== 'state' && d.m[0] !== 'shared') fail(`data-model only accepts state or shared: "${value}"`)
        hooks.on('input'); d.e = 1; break
      case 'c': d.x = 1; break
      case 'self': d.r = 1; break
      case 'page': d.x = 2; break
      case 'slot': d.u = +value
    }
  }
  if (d.r && !el.firstChild) fillSelf(el, el.getAttribute('data-c'))
  // `{path}` beside child elements: <!--{path}-->text<!--}-->; the text node between them is bound.
  // <!--n{path}--> is slot content: it belongs to the owner n components up.
  for (let n = el.firstChild, m; n; n = n.nextSibling) {
    if (n.nodeType !== 8 || !(m = /^(\d*)\{(.*)\}$/.exec(n.data))) continue
    let t = n.nextSibling
    if (t?.nodeType !== 3) n.after((t = document.createTextNode('')))
    d.b.push([5, path(m[2]), t, +m[1]])
  }
  return (el[D] = d.b.length || d.w || d.e || d.x || d.u || d.r ? d : NONE)
}

function bind(el, [t, p, n, a], s) {
  let v
  if (t === 0) {
    if (el.textContent !== (v = fill(s, p))) el.textContent = v
  } else if (t === 4) {
    if (el.getAttribute(a) !== (v = fill(s, p))) el.setAttribute(a, v)
  } else if (t === 5) {
    if (n.data !== (v = str(get(s, p)))) n.data = v
  } else if (t === 6) {
    v = cssValue(get(s, p), a)
    if (el.style.getPropertyValue('--' + a) !== (v ?? '')) v == null ? el.style.removeProperty('--' + a) : el.style.setProperty('--' + a, v)
  } else if (t === 2) {
    v = get(s, p)
    v = v === false || v == null ? null : v === true ? '' : String(v)
    if (el.getAttribute(a) !== v) v == null ? el.removeAttribute(a) : el.setAttribute(a, v)
    // Once the user touches it, the attribute no longer drives the state: set the property.
    if ((a === 'checked' || a === 'selected') && el[a] !== (v != null)) el[a] = v != null
  } else {
    v = !get(s, p) === n
    if (t === 1) { if (el.hasAttribute('hidden') === v) el.toggleAttribute('hidden', !v) }
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
  // data-with-<name>: values from the caller's scope (props of a component with a script).
  let w = null
  if (d.w) {
    w = {}
    for (const [a, p] of d.w) w[a] = 'v' in p ? p.v : get(s, p)
    s = Object.assign({ __proto__: s }, w)
  }
  if (d.r && ++deep > SELF_DEPTH) fail(`<${el.getAttribute('data-self')}> is nested more than ${SELF_DEPTH} levels inside itself: stop the recursion on data (a tree without cycles, <if>/<for> around the self call)`)
  if (d.x === 1) outer.push(s)
  if (d.x) s = hooks.enter(el, s, d.x, w)
  for (const b of d.b) {
    const r = [el, b, b[0] === 5 && b[3] ? (outer.at(-b[3]) ?? s) : s]
    readBy(r)
    bind(...r)
  }
  readBy(0)
  if (d.e) el[S] = s
  if (el.firstElementChild) walk(el, s)
  if (d.m) model(el, s, d.m)
  if (d.x === 1) outer.pop()
  if (d.r) deep--
}

function walk(parent, s) {
  let last, id
  for (let n = parent.firstElementChild; n; n = n.nextElementSibling) {
    if (n[L]) continue
    const d = n[D] ?? parse(n)
    if (d.t) [n, last, id] = block(n, d, lexical(s, d), last, d.id ?? id)
    else visit(n, s, d)
  }
}

// Motion is CSS only: a block root inserted after the block's first render gets `data-entering`
// until its animations end; a removed one gets `data-leaving` + inert and stays until they end.
let leaving = [], entering = [], focusing = null
const reduced = matchMedia('(prefers-reduced-motion: reduce)')

function clone(t, d, id) {
  const n = root(t, d).cloneNode(true)
  n.setAttribute('data-of', id)
  if (d.r && !reduced.matches) {
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

const next = n => {
  do n = n.nextElementSibling
  while (n?.[L])
  return n
}

// else-if/else/empty have no tid of their own: instances use the tid of the if/for opening the chain (same as SSR).
// `last`: an earlier branch of the chain is already on.
function block(t, d, s, last, id) {
  const r = d.t === 2 ? list(t, d, s) : branch(t, d, s, last, id)
  d.r = 1
  return r
}

function branch(t, d, s, last, id) {
  const chained = d.t > 2 && last
  const on = !chained && (d.t === 3 || !get(s, d.p) === d.n)
  let n = t.nextElementSibling
  if (n?.getAttribute('data-of') !== id) n = null
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
  for (let n = t.nextElementSibling; n?.getAttribute('data-of') === id; n = n.nextElementSibling) old.set(n.getAttribute('data-key'), n)
  for (const it of get(s, d.p) ?? []) {
    const c = { __proto__: s, [d.v]: it }
    scopes.push(c)
    keys.push(String(get(c, d.k)))
  }
  const keep = new Set(keys)
  if (self.__pragmaDev && keep.size < keys.length) fail(`duplicate key in <for ${d.v} in ${d.p.join('.')}>`)
  for (const [k, n] of old) if (!keep.has(k)) drop(n)
  let ref = t
  for (let i = 0; i < keys.length; i++) {
    let n = old.get(keys[i])
    if (!n) {
      n = clone(t, d, id)
      n.setAttribute('data-key', keys[i])
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

// After a change read only by bindings (state.js `readers`): runs just those, nothing else is walked.
export function patch(groups) {
  try {
    for (const g of groups) for (const r of g) {
      readBy(r)
      bind(...r)
    }
  } finally { readBy(null) }
}
