const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }

export function escape(value) {
  return String(value).replace(/[&<>"']/g, (c) => ESC[c])
}

export function resolve(scope, path) {
  const parts = path.split('.')
  if (!Object.hasOwn(scope, parts[0])) throw new Error(`unknown path: ${path}`)
  let value = scope[parts[0]]
  for (let i = 1; i < parts.length; i++) {
    if (value == null || !(parts[i] in Object(value))) throw new Error(`unknown path: ${path}`)
    value = value[parts[i]]
  }
  return value
}

const PATH_AT = { text: 1, attr: 2, show: 1, if: 2, elif: 2, for: 3 }
const CLIENT_ROOTS = new Set(['state', 'shared'])

export function clientOnly(scope, path) {
  const root = path.split('.', 1)[0]
  return CLIENT_ROOTS.has(root) && !Object.hasOwn(scope, root)
}

const test = (scope, path, neg) => Boolean(resolve(scope, path)) !== Boolean(neg)

export function render(ops, scope) {
  let out = ''
  // if/elif/else and for/empty chain: true = a branch was rendered, false = none, null = decided on the client.
  let done = null
  for (const op of ops) {
    if (op[0] in PATH_AT && clientOnly(scope, op[PATH_AT[op[0]]])) {
      if (op[0] !== 'text' && op[0] !== 'attr' && op[0] !== 'show') done = null
      continue
    }
    switch (op[0]) {
      case 'html':
        out += op[1]
        break
      case 'text': {
        const v = resolve(scope, op[1])
        out += v == null ? '' : escape(v)
        break
      }
      case 'attr': {
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
        const [kind, tid, path, neg, body] = op
        if (kind === 'elif' && done !== false) break
        done = test(scope, path, neg)
        if (done && body.length) out += instance(render(body, scope), tid)
        break
      }
      case 'else':
      case 'empty': {
        const [, tid, body] = op
        if (done === false && body.length) out += instance(render(body, scope), tid)
        done = null
        break
      }
      case 'for': {
        const [, tid, name, path, keyPath, body] = op
        const list = resolve(scope, path) ?? []
        if (!Array.isArray(list)) throw new Error(`<for> expects a list: ${path}`)
        const seen = new Set()
        for (const item of list) {
          const inner = { ...scope, [name]: item }
          const key = String(resolve(inner, keyPath))
          if (seen.has(key)) throw new Error(`duplicate key in ${path}: ${key}`)
          seen.add(key)
          out += instance(render(body, inner), tid, key)
        }
        done = list.length > 0
        break
      }
      default:
        throw new Error(`unknown op: ${op[0]}`)
    }
  }
  return out
}

function instance(html, tid, key) {
  const root = html.trim()
  const m = /^<[a-zA-Z][^\s/>]*/.exec(root)
  if (!m) throw new Error(`block ${tid} has no root element`)
  const marks = ` data-of="${escape(tid)}"` + (key === undefined ? '' : ` data-key="${escape(key)}"`)
  return m[0] + marks + root.slice(m[0].length)
}

const HEAD_TAG = /<(title|meta|link)\b([^>]*)>(?:([^<]*)<\/title>)?(\n?)/g
const ATTR = /\s([^\s=]+)(?:="([^"]*)")?/g
const BOUND = /^data-(?:attr-(.+)|(text))$/

function lookup(scope, path) {
  try {
    return resolve(scope, path)
  } catch {
    return null
  }
}

export function renderHead(html, scope) {
  const end = html.indexOf('</head>')
  if (end < 0) return html
  const head = html.slice(0, end).replace(HEAD_TAG, (tag, name, raw, text, nl) => {
    const attrs = new Map()
    const bound = []
    for (const [, k, v] of raw.matchAll(ATTR)) {
      const b = BOUND.exec(k)
      if (b) bound.push([b[1] ?? 'text', v])
      else attrs.set(k, v)
    }
    if (!bound.length) return tag
    for (const [attr, path] of bound) {
      const v = lookup(scope, path)
      if (v == null || v === false) continue
      if (attr === 'text') text = escape(v)
      else attrs.set(attr, v === true ? undefined : escape(v))
    }
    if ((name === 'meta' && !attrs.has('content')) || (name === 'link' && !attrs.has('href'))) return ''
    const out = [...attrs].map(([k, v]) => (v === undefined ? ` ${k}` : ` ${k}="${v}"`)).join('')
    return (name === 'title' ? `<title${out}>${text ?? ''}</title>` : `<${name}${out}>`) + nl
  })
  return head + html.slice(end)
}
