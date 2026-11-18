import { initClientLog } from './log.js'
import { startRouter, match } from './router.js'
import { importRoute, enter } from './page.js'
import { fetchData } from './loader.js'

const route = JSON.parse(document.getElementById('pragma-route').textContent)
const url = new URL(location.href)
const pending = route.loader && !route.ssr ? self.__pd ?? fetchData(url) : undefined
self.__pd = null

initClientLog({ endpoint: '/_otel', service: location.hostname })
enter(route, await importRoute(route), url, match(route.pattern, url.pathname) ?? {}, document, pending)
startRouter()
