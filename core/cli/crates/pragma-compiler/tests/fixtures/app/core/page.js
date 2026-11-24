const installed = new WeakSet()
export const K = {}

export function install(use = []) {
  for (const f of use) if (!installed.has(f)) installed.add(f), f(K)
}

export function start(R) {
  install(R.use)
  import('./log.js').then(({ log }) => import('./router.js').then(r => r.start(R, log)))
}
