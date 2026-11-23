import { initClientLog } from './log.js'
import { importRoute, enter, leave, closeOverlays, popOverlay, loadLayout, router } from './page.js'
import { fetchData, fetchLayout } from './loader.js'
import { ROUTE_ID } from './contract.js'

const route = JSON.parse(document.getElementById(ROUTE_ID).textContent)
const url = new URL(location.href)
const pending = route.loader && !route.ssr ? self.__pd ?? fetchData(url) : undefined
self.__pd = null
const layoutDone = route.layout ? loadLayout(route.ssr ? null : self.__pl ?? fetchLayout(url), document) : null
self.__pl = null

initClientLog(location.hostname)
importRoute(route).then(mods => {
  const loaded = Promise.all([enter(route, mods, url, document, pending), layoutDone]).catch(() => {})
  router().then(r => r.startRouter({ importRoute, enter, leave, closeOverlays, popOverlay, loaded }))
})
