import { serve, registerNodeHooks } from './core/server/node.mjs'

registerNodeHooks()
const { default: h } = await import('./worker.js')
serve(h, { root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000, service: "test-shop" })
