const TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/
const SAME_SITE = { lax: 'Lax', strict: 'Strict', none: 'None' }
const LOCAL_HOSTS = ['localhost', '127.0.0.1', '[::1]']

export function parseCookies(header) {
  const out = {}
  for (const part of (header ?? '').split(';')) {
    const i = part.indexOf('=')
    if (i < 0) continue
    const name = part.slice(0, i).trim()
    if (!name || Object.hasOwn(out, name)) continue
    out[name] = decode(part.slice(i + 1).trim().replace(/^"(.*)"$/, '$1'))
  }
  return out
}

function decode(value) {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

export function createCookies(request, url) {
  const incoming = parseCookies(request.headers.get('cookie'))
  const outgoing = new Map()
  const insecureLocal = url.protocol === 'http:' && LOCAL_HOSTS.includes(url.hostname)

  function set(name, value, options = {}) {
    if (typeof name !== 'string' || !TOKEN.test(name)) {
      throw new Error(`cookies.set(${JSON.stringify(name)}): invalid cookie name; use letters, digits, - and _`)
    }
    const opts = { path: '/', httpOnly: true, sameSite: 'lax', secure: !insecureLocal, ...options }
    if (name.startsWith('__Host-')) {
      if (opts.domain) throw new Error(`cookies.set("${name}"): __Host- cookies cannot have a domain; remove the domain option`)
      if (opts.path !== '/') throw new Error(`cookies.set("${name}"): __Host- cookies need path "/"; remove the path option`)
      opts.secure = true
    }
    if (name.startsWith('__Secure-')) opts.secure = true
    const sameSite = SAME_SITE[String(opts.sameSite).toLowerCase()]
    if (opts.sameSite !== false && !sameSite) {
      throw new Error(`cookies.set("${name}"): sameSite must be "lax", "strict" or "none"`)
    }
    if (sameSite === 'None') opts.secure = true
    let line = `${name}=${encodeURIComponent(String(value))}`
    if (opts.maxAge !== undefined) line += `; Max-Age=${Math.floor(opts.maxAge)}`
    if (opts.expires) line += `; Expires=${new Date(opts.expires).toUTCString()}`
    if (opts.domain) line += `; Domain=${opts.domain}`
    if (opts.path) line += `; Path=${opts.path}`
    if (opts.httpOnly) line += '; HttpOnly'
    if (opts.secure) line += '; Secure'
    if (sameSite && opts.sameSite !== false) line += `; SameSite=${sameSite}`
    if (opts.partitioned) line += '; Partitioned'
    const removed = opts.maxAge === 0 || (opts.expires && new Date(opts.expires).getTime() <= Date.now())
    outgoing.set(`${opts.domain ?? ''}${opts.path ?? ''}?${name}`, { name, value: removed ? undefined : String(value), line })
  }

  function latest(name) {
    let found
    for (const cookie of outgoing.values()) if (cookie.name === name) found = cookie
    return found
  }

  const cookies = {
    get(name) {
      const set = latest(name)
      return set ? set.value : incoming[name]
    },
    getAll() {
      const all = { ...incoming }
      for (const cookie of outgoing.values()) {
        if (cookie.value === undefined) delete all[cookie.name]
        else all[cookie.name] = cookie.value
      }
      return Object.entries(all).map(([name, value]) => ({ name, value }))
    },
    set,
    delete(name, options = {}) {
      set(name, '', { ...options, maxAge: 0 })
    },
  }
  return { cookies, setCookieLines: () => [...outgoing.values()].map((cookie) => cookie.line) }
}
