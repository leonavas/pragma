import { describe, it, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { start } from './app.mjs'
import { appSuite } from './suite.mjs'

const PORT = Number(process.env.PORT ?? 5281)
let app

before(async () => { app = await start('serve', PORT) })
after(() => app?.stop())

appSuite('serve', () => app)

describe('pragma serve build output', () => {
  const name = href => href.split('/assets/')[1].replace(/-[0-9a-f]{8}\.js$/, '')
  const firstLoad = async url => {
    const html = await (await fetch(app.url + url)).text()
    const preloads = [...html.matchAll(/rel="modulepreload" href="([^"]+)"/g)].map(m => name(m[1]))
    const entry = html.match(/<script type="module" src="([^"]+)"/)?.[1]
    return { entry: entry && name(entry), preloads }
  }
  const LAZY = ['chunks/overlay', 'chunks/router', 'chunks/log', 'chunks/action']

  it('each page loads only its own entry and kernel; lazy chunks are never preloaded', async () => {
    const dashboard = await firstLoad('/')
    assert.equal(dashboard.entry, 'routes/index')
    assert.ok(dashboard.preloads.includes('chunks/page'))
    assert.ok(!dashboard.preloads.some(chunk => chunk.startsWith('routes/') || LAZY.includes(chunk)))
    const shop = await firstLoad('/shop')
    assert.equal(shop.entry, 'routes/shop/index')
    assert.ok(!shop.preloads.some(chunk => chunk.startsWith('routes/') || LAZY.includes(chunk)))
    const notFound = await (await fetch(app.url + '/_404.html')).text()
    assert.ok(!notFound.includes('overlay-'))
  })

  it('hashed assets are immutable; no dev code in the build', async () => {
    const html = await (await fetch(app.url + '/')).text()
    assert.doesNotMatch(html, /__pragmaDev|\/_pragma\/dev\.js/)
    const asset = html.match(/href="(\/assets\/[^"]+-[0-9a-f]{8}\.js)"/)[1]
    const response = await fetch(app.url + asset)
    assert.equal(response.status, 200)
    assert.match(response.headers.get('cache-control') ?? '', /immutable/)
  })
})
