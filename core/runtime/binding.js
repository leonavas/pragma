export function coerce(v, d) {
  if (v === undefined) return d
  switch (typeof d) {
    case 'number': return typeof v === 'number' ? v : v === '' ? d : Number(v)
    case 'boolean': return v === true || v === '' || v === 'true' ? true : v === false || v === 'false' ? false : Boolean(v)
    case 'string': return v == null ? d : String(v)
    default: return v
  }
}

const TEMPLATE = /\\\{|\{([^{}]*)\}/g

export function template(value) {
  const parts = ['']
  let at = 0
  for (const m of value.matchAll(TEMPLATE)) {
    parts[parts.length - 1] += value.slice(at, m.index)
    if (m[1] == null) parts[parts.length - 1] += '{'
    else parts.push(m[1], '')
    at = m.index + m[0].length
  }
  parts[parts.length - 1] += value.slice(at)
  return parts
}

export function lookup(scope, parts, missing) {
  let value = scope
  for (const key of parts) {
    const owner = value
    if (owner == null || !(key in Object(owner))) return parts[0] === 'query' ? undefined : missing(parts.join('.'))
    value = owner[key]
    if (typeof value === 'function') value = value.call(owner)
  }
  return value
}

export function cssValue(v, unsafe) {
  if (v == null || v === false || v === '') return null
  v = String(v)
  return /[;{}<]/.test(v) ? unsafe(v) : v
}

export function readonlyViews(wraps, message) {
  const views = new WeakMap()
  const view = (value, name) => {
    if (value === null || typeof value !== 'object' || !wraps(value)) return value
    let proxy = views.get(value)
    if (!proxy) {
      const deny = () => {
        throw new TypeError(message(name))
      }
      const get = (t, k, r) => (typeof k === 'symbol' ? Reflect.get(t, k, r) : view(Reflect.get(t, k, r), name))
      proxy = new Proxy(value, { get, set: deny, deleteProperty: deny, defineProperty: deny })
      views.set(value, proxy)
    }
    return proxy
  }
  return view
}

export function instantiate(mod, target, scope, services, propValue) {
  const prop = (name, d) => {
    const key = name.toLowerCase()
    Object.defineProperty(target, name, { enumerable: true, get: () => propValue(key, d, name) })
  }
  const api = mod.default?.(scope, { __proto__: services, prop }) ?? {}
  for (const k in api) if (k !== 'mount') target[k] = api[k]
  return api.mount
}
