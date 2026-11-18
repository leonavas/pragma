import { start } from './router.js'
import { log } from './log.js'

const route = JSON.parse(document.getElementById('pragma-route').textContent)
start(route, log)
