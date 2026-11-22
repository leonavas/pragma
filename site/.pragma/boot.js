import { initClientLog } from './log.js'
import { startRouter } from './router.js'
import { importRoute, enter, leave, closeOverlays, popOverlay } from './page.js'
import { fetchData } from './loader.js'
import { ROUTE_ID } from './contract.js'

const route = JSON.parse(document.getElementById(ROUTE_ID).textContent)
const url = new URL(location.href)
const pending = route.loader && !route.ssr ? self.__pd ?? fetchData(url) : undefined
self.__pd = null

initClientLog(location.hostname)
importRoute(route).then(mods => {
  enter(route, mods, url, document, pending)
  startRouter({ importRoute, enter, leave, closeOverlays, popOverlay })
})
