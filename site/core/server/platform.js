export function createDb(binding) {
  if (!binding) return undefined
  if (typeof binding.get === 'function' && typeof binding.all === 'function' && typeof binding.run === 'function') {
    return binding
  }
  if (typeof binding.prepare !== 'function') return undefined
  const stmt = (sql, params) => {
    const q = binding.prepare(sql)
    const args = params.length === 1 && Array.isArray(params[0]) ? params[0] : params
    return args.length ? q.bind(...args) : q
  }
  return {
    async get(sql, ...params) {
      return (await stmt(sql, params).first()) ?? null
    },
    async all(sql, ...params) {
      const res = await stmt(sql, params).all()
      return Array.isArray(res) ? res : (res?.results ?? [])
    },
    async run(sql, ...params) {
      return stmt(sql, params).run()
    },
  }
}

export function createKv(binding) {
  if (!binding) return undefined
  if (typeof binding.get === 'function' && typeof binding.put === 'function' && typeof binding.delete === 'function') {
    return binding
  }
  if (typeof binding.get === 'function' && typeof binding.put === 'function' && typeof binding.del === 'function') {
    return {
      get: (key, opts) => binding.get(key, opts),
      put: (key, value, opts) => binding.put(key, value, opts),
      delete: (key) => binding.del(key),
    }
  }
  return undefined
}

export function createFiles(binding) {
  if (!binding) return undefined
  if (typeof binding.get === 'function' && typeof binding.put === 'function' && typeof binding.delete === 'function') {
    return binding
  }
  return undefined
}

export function createPlatform(env) {
  return {
    db: createDb(env?.DB),
    kv: createKv(env?.KV),
    files: createFiles(env?.FILES ?? env?.BUCKET),
  }
}
