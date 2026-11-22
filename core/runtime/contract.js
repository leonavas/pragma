const DATA = '/_data'
const ACTION = '/_action'
export const OTEL = '/_otel'
export const PRAGMA = '/_pragma/'
export const MANIFEST = PRAGMA + 'manifest.json'
export const ASSETS = '/assets/'
export const IMMUTABLE = 'public, max-age=31536000, immutable'

export const ROUTE_ID = 'pragma-route'
export const DATA_ID = 'pragma-data'
export const MAIN = 'main[data-page]'
export const OF = 'data-of'
export const KEY = 'data-key'
export const ROUTE_CSS = 'data-pragma-route-css'
export const SHARED_CSS = 'data-pragma-shared-css'
export const LAYOUT_PREFIX = 'l'
export const PERSIST_SUFFIX = '$persist'
export const THEME_KEY = 'pragma:theme'
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

export const matchPattern = (pattern, path) => matchRoute(sortRoutes([{ pattern }]), path)?.params ?? null

export const pageUrl = file => encodeURI(file.replace(/(\/index)?\.html$/, '')) || '/'

export const dataUrl = url => DATA + url.pathname + url.search

export const actionUrl = (url, name) => `${ACTION}${url.pathname.replace(/\/$/, '')}/${name}`

export function apiTarget(path) {
  if (path === DATA || path.startsWith(DATA + '/')) return { kind: 'data', target: path.slice(DATA.length) || '/' }
  if (!path.startsWith(ACTION + '/')) return null
  const rest = path.slice(ACTION.length), i = rest.lastIndexOf('/')
  return { kind: 'action', target: rest.slice(0, i) || '/', action: rest.slice(i + 1) }
}
