export function start(R, log) {
  log.info('route', { pattern: R.pattern })
  return import('./lazy.js')
}
