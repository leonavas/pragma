export function start(route, log) {
  log.info('route', { pattern: route.pattern })
  return import('./lazy.js')
}
