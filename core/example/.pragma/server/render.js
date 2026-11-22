import { coerce, cssValue as safeCss, instantiate as runScript, lookup, readonlyViews, template } from '../binding.js'
import { KEY, OF } from '../contract.js'

const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }

function escape(value) {
  return String(value).replace(/[&<>"']/g, (c) => ESC[c])
}

const unknown = (path) => {
  throw new Error(`unknown path: ${path}`)
}

const resolve = (scope, path) => lookup(scope, path.split('.'), unknown)

const isTemplate = (value) => value.includes('{')
const pathsOf = (value) => (isTemplate(value) ? template(value).filter((_, i) => i % 2) : [value])

function fill(scope, value, literal = (text) => text) {
  const parts = template(value)
  let out = literal(parts[0])
  for (let i = 1; i < parts.length; i += 2) {
    const v = resolve(scope, parts[i])
    out += (v == null ? '' : escape(v)) + literal(parts[i + 1])
  }
  return out
}

const quoted = (text) => text.replace(/"/g, '&quot;')

const CSS_VAR_END = /(?:^|;)--[a-z0-9-]+:$/
const cssValue = (v) => safeCss(v, () => null)

function style(scope, value) {
  const parts = template(value)
  let out = parts[0]
  for (let i = 1; i < parts.length; i += 2) {
    const v = resolve(scope, parts[i])
    if (!CSS_VAR_END.test(out)) out += v == null ? '' : escape(v)
    else if (cssValue(v) == null) out = out.replace(CSS_VAR_END, '')
    else out += escape(cssValue(v))
    out += parts[i + 1]
  }
  return quoted(out.replace(/^;/, ''))
}

const PATH_AT = { text: 1, attr: 2, show: 1, if: 2, elif: 2, for: 3 }
const CLIENT_ROOTS = new Set(['state', 'shared'])

function clientOnly(scope, value) {
  return pathsOf(value).some((path) => {
    const root = path.split('.', 1)[0]
    return CLIENT_ROOTS.has(root) && !Object.hasOwn(scope, root)
  })
}

const test = (scope, path, neg) => Boolean(resolve(scope, path)) !== Boolean(neg)

const scopeOf = (values) => ({ __proto__: null, ...values })

export function render(ops, scope, ctx = {}) {
  return run(ops, scopeOf(scope), { outer: [], instance: () => null, ...ctx }, { done: null })
}

// `chain.done` (if/elif/else and for/empty): true = a branch was rendered, false = none, null = decided
// on the client. A `slot` op shares its siblings' chain.
function run(ops, scope, ctx, chain) {
  let out = ''
  for (const op of ops) {
    if (op[0] in PATH_AT && clientOnly(scope, op[PATH_AT[op[0]]])) {
      if (op[0] !== 'text' && op[0] !== 'attr' && op[0] !== 'show') chain.done = null
      continue
    }
    switch (op[0]) {
      case 'html':
        out += op[1]
        break
      case 'text':
        out += fill(scope, op[1])
        break
      case 'attr': {
        if (op[1] === 'style' && isTemplate(op[2])) {
          const css = style(scope, op[2])
          if (css) out += ` style="${css}"`
          break
        }
        if (isTemplate(op[2])) {
          out += ` ${op[1]}="${fill(scope, op[2], quoted)}"`
          break
        }
        const v = resolve(scope, op[2])
        if (v === true) out += ` ${op[1]}=""`
        else if (v !== false && v != null) out += ` ${op[1]}="${escape(v)}"`
        break
      }
      case 'class': {
        const names = op[1] ? op[1].split(/\s+/).filter(Boolean) : []
        for (const [name, path, neg] of op[2]) if (!clientOnly(scope, path) && test(scope, path, neg)) names.push(name)
        if (names.length) out += ` class="${escape(names.join(' '))}"`
        break
      }
      case 'show':
        if (!test(scope, op[1], op[2])) out += ' hidden'
        break
      case 'if':
      case 'elif': {
        const [kind, id, path, neg, body] = op
        if (kind === 'elif' && chain.done !== false) break
        chain.done = test(scope, path, neg)
        if (chain.done && body.length) out += instance(run(body, scope, ctx, { done: null }), id)
        break
      }
      case 'else':
      case 'empty': {
        const [, id, body] = op
        if (chain.done === false && body.length) out += instance(run(body, scope, ctx, { done: null }), id)
        chain.done = null
        break
      }
      case 'for': {
        const [, id, name, path, keyPath, body] = op
        const list = resolve(scope, path) ?? []
        if (!Array.isArray(list)) throw new Error(`<for> expects a list: ${path}`)
        const seen = new Set()
        for (const item of list) {
          const inner = scopeOf({ ...scope, [name]: item })
          const key = String(resolve(inner, keyPath))
          if (seen.has(key)) throw new Error(`duplicate key in ${path}: ${key}`)
          seen.add(key)
          out += instance(run(body, inner, ctx, { done: null }), id, key)
        }
        chain.done = list.length > 0
        break
      }
      case 'comp':
        out += component(op, scope, ctx)
        chain.done = null
        break
      case 'slot':
        out += run(op[2], ctx.outer.at(-op[1]) ?? scope, ctx, chain)
        break
      default:
        throw new Error(`unknown op: ${op[0]}`)
    }
  }
  return out
}

function component([, id, props, body], scope, ctx) {
  const { state: _, ...outside } = scope
  const values = {}
  let known = true
  for (const [name, kind, value] of props) {
    if (kind === 'text') values[name] = value
    else if (clientOnly(scope, value)) known = false
    else values[name] = resolve(scope, value)
  }
  ctx.outer.push(scope)
  try {
    const state = known ? ctx.instance(id, values) : null
    if (state) {
      try {
        return run(body, scopeOf({ ...outside, state }), ctx, { done: null })
      } catch {}
    }
    return run(body, scopeOf(outside), ctx, { done: null })
  } finally {
    ctx.outer.pop()
  }
}

const browserOnly = (name) => () => {
  throw new Error(`${name} runs only in the browser`)
}

const readonly = readonlyViews(
  () => true,
  () => 'data, params, query and page are read-only while the server renders',
)

export function serverServices({ data, params, query, page }, log) {
  return {
    data: readonly(data), params: readonly(params), query: readonly(query), page: readonly(page), log,
    shared: new Proxy({}, { get: browserOnly('shared') }),
    call: browserOnly('call'), stream: browserOnly('stream'), frame: browserOnly('frame'), navigate: browserOnly('navigate'), open: browserOnly('open'), close: browserOnly('close'), confirm: browserOnly('confirm'),
  }
}

function instantiate(mod, props, services) {
  if (typeof mod?.default !== 'function') return null
  const target = {}
  try {
    runScript(mod, target, target, services, (key, d) => coerce(Object.hasOwn(props, key) ? props[key] : undefined, d))
    return target
  } catch {
    return null
  }
}

export function renderPage(ops, scope, { page, components = {}, log } = {}) {
  const services = serverServices(scope, log)
  const ctx = { instance: (id, props) => instantiate(components[id], props, services) }
  const state = instantiate(page, {}, services)
  if (state) {
    try {
      return render(ops, { ...scope, state }, ctx)
    } catch {}
  }
  return render(ops, scope, ctx)
}

function instance(html, id, key) {
  const root = html.trim()
  const m = /^<[a-zA-Z][^\s/>]*/.exec(root)
  if (!m) throw new Error(`block ${id} has no root element`)
  const marks = ` ${OF}="${escape(id)}"` + (key === undefined ? '' : ` ${KEY}="${escape(key)}"`)
  return m[0] + marks + root.slice(m[0].length)
}

const HEAD_TAG = /<(title|meta|link)\b([^>]*)>(?:([^<]*)<\/title>)?(\n?)/g
const ATTR = /\s([^\s=]+)(?:="([^"]*)")?/g
const BOUND = /^data-(?:attr-(.+)|(text))$/

function headValue(scope, value) {
  try {
    if (isTemplate(value)) return fill(scope, value)
    const v = resolve(scope, value)
    return v == null || v === false ? null : v === true ? true : escape(v)
  } catch {
    return null
  }
}

const JSON_LD = /(<script\b[^>]*\btype="application\/ld\+json"[^>]*>)([^]*?)(<\/script>)/gi

// A value inside a JSON string: JSON-escaped, and `<` too, so it can never close the <script>.
function jsonValue(scope, path) {
  try {
    const v = resolve(scope, path)
    return v == null ? '' : JSON.stringify(String(v)).slice(1, -1).replace(/</g, '\\u003c')
  } catch {
    return ''
  }
}

function fillJsonLd(body, scope) {
  let out = ''
  let inString = false
  for (let i = 0; i < body.length; i++) {
    const c = body[i]
    if (inString && c === '\\') {
      out += c + (body[++i] ?? '')
      continue
    }
    if (c === '"') inString = !inString
    else if (inString && c === '{') {
      const close = body.indexOf('}', i)
      const quote = body.indexOf('"', i)
      if (close > 0 && (quote < 0 || close < quote)) {
        out += jsonValue(scope, body.slice(i + 1, close).trim())
        i = close
        continue
      }
    }
    out += c
  }
  return out
}

export function renderHead(html, values) {
  const end = html.indexOf('</head>')
  if (end < 0) return html
  const scope = scopeOf(values)
  const head = html.slice(0, end).replace(JSON_LD, (tag, open, body, close) => open + fillJsonLd(body, scope) + close).replace(HEAD_TAG, (tag, name, raw, text, nl) => {
    const attrs = new Map()
    const bound = []
    for (const [, k, v] of raw.matchAll(ATTR)) {
      const b = BOUND.exec(k)
      if (b) bound.push([b[1] ?? 'text', v])
      else attrs.set(k, v)
    }
    if (!bound.length) return tag
    for (const [attr, value] of bound) {
      const v = headValue(scope, value)
      if (v == null) continue
      if (attr === 'text') text = v === true ? '' : v
      else attrs.set(attr, v === true ? undefined : v)
    }
    if ((name === 'meta' && !attrs.has('content')) || (name === 'link' && !attrs.has('href'))) return ''
    const out = [...attrs].map(([k, v]) => (v === undefined ? ` ${k}` : ` ${k}="${v}"`)).join('')
    return (name === 'title' ? `<title${out}>${text ?? ''}</title>` : `<${name}${out}>`) + nl
  })
  return head + html.slice(end)
}
