export function start(route, log) {
  log.info('rota', { pattern: route.pattern })
  return import('./lazy.js')
}
