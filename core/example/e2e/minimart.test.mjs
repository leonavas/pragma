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
  const preloads = async url => (await (await fetch(app.url + url)).text()).match(/rel="modulepreload" href="[^"]+"/g).map(tag => tag.split('/assets/')[1].replace(/-[0-9a-f]{8}\.js"$/, ''))

  it('each page preloads only what the first paint uses', async () => {
    const dashboard = await preloads('/')
    assert.ok(dashboard.includes('core/overlay') && dashboard.includes('lib/components/clock/clock'))
    assert.ok(!dashboard.some(module => module.startsWith('routes/shop')))
    const shop = await preloads('/shop')
    assert.ok(!shop.includes('lib/components/clock/clock') && !shop.includes('core/theme'))
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
