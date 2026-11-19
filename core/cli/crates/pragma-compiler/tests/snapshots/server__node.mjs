import h from './worker.js'
import { serve } from './core/server/node.mjs'

serve(h, { root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000, service: "test-shop" })
