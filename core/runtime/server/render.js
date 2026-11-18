const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }

export function escape(value) {
  return String(value).replace(/[&<>"']/g, (c) => ESC[c])
}

export function resolve(scope, path) {
  const parts = path.split('.')
  if (!Object.hasOwn(scope, parts[0])) throw new Error(`caminho inexistente: ${path}`)
  let value = scope[parts[0]]
  for (let i = 1; i < parts.length; i++) {
    if (value == null || !(parts[i] in Object(value))) throw new Error(`caminho inexistente: ${path}`)
    value = value[parts[i]]
  }
  return value
}

const PATH_AT = { text: 1, attr: 2, show: 1, if: 2, for: 3 }
const CLIENT_ROOTS = new Set(['state', 'shared'])

export function clientOnly(scope, path) {
  const root = path.split('.', 1)[0]
  return CLIENT_ROOTS.has(root) && !Object.hasOwn(scope, root)
}

const test = (scope, path, neg) => Boolean(resolve(scope, path)) !== Boolean(neg)

export function render(ops, scope) {
  let out = ''
  for (const op of ops) {
    if (op[0] in PATH_AT && clientOnly(scope, op[PATH_AT[op[0]]])) continue
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
      case 'if': {
        const [, tid, path, neg, yes, no] = op
        const branch = test(scope, path, neg) ? yes : no
        if (branch?.length) out += instance(render(branch, scope), tid)
        break
      }
      case 'for': {
        const [, tid, name, path, keyPath, body, empty] = op
        const list = resolve(scope, path) ?? []
        if (!Array.isArray(list)) throw new Error(`data-for espera uma lista: ${path}`)
        if (!list.length) {
          if (empty?.length) out += instance(render(empty, scope), tid)
          break
        }
        const seen = new Set()
        for (const item of list) {
          const inner = { ...scope, [name]: item }
          const key = String(resolve(inner, keyPath))
          if (seen.has(key)) throw new Error(`data-key repetido em ${path}: ${key}`)
          seen.add(key)
          out += instance(render(body, inner), tid, key)
        }
        break
      }
      default:
        throw new Error(`op desconhecida: ${op[0]}`)
    }
  }
  return out
}

function instance(html, tid, key) {
  const root = html.trim()
  const m = /^<[a-zA-Z][^\s/>]*/.exec(root)
  if (!m) throw new Error(`bloco ${tid} sem elemento raiz`)
  const marks = ` data-of="${escape(tid)}"` + (key === undefined ? '' : ` data-key="${escape(key)}"`)
  return m[0] + marks + root.slice(m[0].length)
}
