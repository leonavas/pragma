const DATA = '/_data'
const ACTION = '/_action'
const LAYOUT = '/_layout'
export const OTEL = '/_otel'
export const PRAGMA = '/_pragma/'
export const MANIFEST = PRAGMA + 'manifest.json'
export const ASSETS = '/assets/'
export const IMMUTABLE = 'public, max-age=31536000, immutable'

export const DATA_ID = 'pragma-data'
export const MAIN = 'main[data-page]'
export const OF = 'data-of'
export const KEY = 'data-key'
export const ROUTE_CSS = 'data-pragma-route-css'
export const SHARED_CSS = 'data-pragma-shared-css'
export const LAYOUT_PREFIX = 'l'
export const STORE_KEY = name => (name === 'theme' ? 'pragma:store:theme' : 'pragma:' + name)

const segments = path => path.split('/').filter(Boolean)

export function sortRoutes(routes) {
  return routes
    .map(route => {
      const segs = segments(route.pattern)
      return { route, segs, rank: segs.map(s => (s[0] === ':' ? '1' : '0')).join('') }
    })
    .sort((a, b) => (a.rank < b.rank ? -1 : a.rank > b.rank ? 1 : 0))
}

function paramsOf(segs, parts) {
  if (segs.length !== parts.length) return null
  const params = {}
  for (let i = 0; i < segs.length; i++) {
    if (segs[i][0] === ':') params[segs[i].slice(1)] = parts[i]
    else if (segs[i] !== parts[i]) return null
  }
  return params
}

function decodedSegments(path) {
  try {
    return segments(path).map(decodeURIComponent)
  } catch {
    return null
  }
}

export function matchRoute(table, path) {
  const parts = decodedSegments(path)
  if (!parts) return null
  for (const { route, segs } of table) {
    const params = paramsOf(segs, parts)
    if (params) return { route, params }
  }
  return null
}

export function matchPattern(pattern, path) {
  const parts = decodedSegments(path)
  return parts && paramsOf(segments(pattern), parts)
}

export const pageUrl = file => encodeURI(file.replace(/(\/index)?\.html$/, '')) || '/'

export const dataUrl = url => DATA + url.pathname + url.search

export const layoutUrl = url => LAYOUT + url.pathname + url.search

export const actionUrl = (url, name) => `${ACTION}${url.pathname.replace(/\/$/, '')}/${name}`

export function apiTarget(path) {
  if (path === DATA || path.startsWith(DATA + '/')) return { kind: 'data', target: path.slice(DATA.length) || '/' }
  if (path === LAYOUT || path.startsWith(LAYOUT + '/')) return { kind: 'layout', target: path.slice(LAYOUT.length) || '/' }
  if (!path.startsWith(ACTION + '/')) return null
  const rest = path.slice(ACTION.length), i = rest.lastIndexOf('/')
  return { kind: 'action', target: rest.slice(0, i) || '/', action: rest.slice(i + 1) }
}

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
    if (owner == null) return undefined
    if (!(key in Object(owner))) return parts[0] === 'query' ? undefined : missing(parts.join('.'))
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

export function isPlain(v) {
  const proto = Object.getPrototypeOf(v)
  return proto === Object.prototype || proto === null || Array.isArray(v)
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
