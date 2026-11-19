import { test, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { start } from './server.mjs'

const tools = process.env.PRAGMA_TOOLS ?? '/tmp/pragma-tools-runtime-client'
const { chromium } = createRequire(tools + '/')('playwright')

let server, browser

before(async () => {
  server = await start()
  browser = await chromium.launch()
})
after(async () => {
  await browser?.close()
  server?.close()
})

async function open(path) {
  const ctx = await browser.newContext()
  const page = await ctx.newPage()
  page.errors = []
  page.requests = []
  page.on('pageerror', e => page.errors.push(e.message))
  page.on('request', r => page.requests.push(new URL(r.url()).pathname))
  await page.goto(server.url + path)
  await page.waitForFunction(() => document.querySelector('#qty')?.textContent !== '')
  return page
}

const text = (page, sel) => page.locator(sel).textContent()
const tick = page => page.evaluate(() => new Promise(r => setTimeout(r)))

test('data-text, data-attr, data-class, data-show', async () => {
  const p = await open('/directives')
  assert.equal(await text(p, '#name'), 'Ana')
  const link = await p.evaluate(() => {
    const a = document.querySelector('#link')
    return [a.getAttribute('href'), a.getAttribute('aria-disabled'), a.hasAttribute('title'), a.getAttribute('data-x')]
  })
  assert.deepEqual(link, ['/x', null, false, ''])
  assert.equal(await p.getAttribute('#cls', 'class'), 'base active')
  assert.equal(await p.isVisible('#show'), true)
  assert.equal(await p.isVisible('#hide'), false)

  await p.evaluate(() => __ctx.update(s => { s.active = false; s.title = 'hi'; s.disabled = true; s.name = '<b>x</b>' }))
  assert.equal(await text(p, '#name'), '<b>x</b>')
  assert.equal(await p.locator('#name b').count(), 0)
  const link2 = await p.evaluate(() => {
    const a = document.querySelector('#link')
    return [a.getAttribute('aria-disabled'), a.getAttribute('title'), a.hasAttribute('data-x')]
  })
  assert.deepEqual(link2, ['', 'hi', false])
  assert.equal(await p.getAttribute('#cls', 'class'), 'base inactive')
  assert.equal(await p.isVisible('#show'), false)
  assert.equal(await p.isVisible('#hide'), true)
  assert.deepEqual(p.errors, [])
})

test('data-if/data-else, data-with and writes only on change', async () => {
  const p = await open('/directives')
  assert.equal(await p.locator('#yes').count(), 1)
  assert.equal(await p.locator('#no').count(), 0)
  assert.equal(await p.getAttribute('#yes', 'data-of'), 't1')
  await p.click('#toggle')
  assert.equal(await p.locator('#yes').count(), 0)
  assert.equal(await p.getAttribute('#no', 'data-of'), 't1')
  assert.equal(await text(p, '#w'), 'Bia')
  const writes = await p.evaluate(async () => {
    const seen = []
    const mo = new MutationObserver(rs => seen.push(...rs))
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true })
    __ctx.update(() => {})
    await new Promise(r => setTimeout(r))
    mo.disconnect()
    return seen.length
  })
  assert.equal(writes, 0)
})

test('nested data-for/data-empty and keyed diff preserving nodes', async () => {
  const p = await open('/directives')
  const items = () => p.$$eval('#list > li[data-of="t2"][data-key]', ls => ls.map(l => [l.dataset.key, l.querySelector('.n').textContent, l.querySelectorAll('.ok').length, [...l.querySelectorAll('.tag')].map(t => t.textContent).join(), l.querySelectorAll('.none').length]))
  assert.deepEqual(await items(), [['1', 'one', 1, 'a,b', 0], ['2', 'two', 0, '', 1]])

  await p.evaluate(() => {
    self.__li = [...document.querySelectorAll('#list > li[data-of]')]
    self.__tag = document.querySelector('.tag')
    __ctx.update(s => {
      s.items.unshift({ id: 3, name: 'three', ok: true, tags: [{ t: 'z' }] })
      s.items.reverse()
      s.items[1].tags.unshift({ t: 'c' })
      s.items[1].name = 'ONE'
    })
  })
  assert.deepEqual(await items(), [['2', 'two', 0, '', 1], ['1', 'ONE', 1, 'c,a,b', 0], ['3', 'three', 1, 'z', 0]])
  const same = await p.evaluate(() => {
    const now = [...document.querySelectorAll('#list > li[data-of]')]
    return [now[0] === __li[1], now[1] === __li[0], document.querySelectorAll('.tag')[1] === __tag]
  })
  assert.deepEqual(same, [true, true, true])

  await p.evaluate(() => __ctx.update(s => { s.items = [] }))
  assert.deepEqual(await items(), [])
  assert.equal(await p.locator('#empty').count(), 1)
  await p.evaluate(() => __ctx.update(s => { s.items = [{ id: 9, name: 'nine', ok: false, tags: [] }] }))
  assert.equal(await p.locator('#empty').count(), 0)
  assert.deepEqual(p.errors, [])
})

test('components: per-instance state, owner action and cleanup on removal', async () => {
  const p = await open('/directives')
  assert.equal(await p.evaluate(() => self.__mounted), 2)
  // mount(el) receives the component root; the page's mount gets <main data-page>
  assert.deepEqual(await p.evaluate(() => self.__mountRoots), ['counter', 'counter'])
  assert.equal(await p.evaluate(() => self.__pageRoot), 'MAIN')
  await p.locator('#list li[data-key="1"] .inc').click()
  await p.locator('#list li[data-key="1"] .inc').click()
  await p.locator('#list li[data-key="2"] .inc').click()
  assert.deepEqual(await p.$$eval('.count', e => e.map(x => x.textContent)), ['2', '1'])
  assert.deepEqual(await p.$$eval('.owner', e => e.map(x => x.textContent)), ['one', 'two'])
  await p.locator('#list li[data-key="1"] .rm').click()
  assert.deepEqual(await p.$$eval('.count', e => e.map(x => x.textContent)), ['1'])
  assert.equal(await p.evaluate(() => self.__unmounted), 1)
})

test('data-model: text, checkbox, number, select and radio', async () => {
  const p = await open('/directives')
  assert.equal(await p.inputValue('#m-text'), 'Ana')
  assert.equal(await p.inputValue('#m-sel'), 'green')
  assert.equal(await p.isChecked('#r-p'), true)
  await p.fill('#m-text', 'Carla')
  assert.equal(await text(p, '#name'), 'Carla')
  await p.check('#m-check')
  assert.equal(await text(p, '#echo-accepted'), 'true')
  await p.fill('#m-num', '7')
  assert.equal(await text(p, '#echo-qty'), '7')
  assert.equal(await p.evaluate(() => typeof __ctx.$.qty), 'number')
  await p.selectOption('#m-sel', 'purple')
  assert.equal(await text(p, '#echo-color'), 'purple')
  await p.check('#r-g')
  assert.equal(await text(p, '#echo-size'), 'g')
  await p.evaluate(() => __ctx.update(s => { s.name = 'Dani'; s.accepted = false; s.size = 'p'; s.color = 'blue' }))
  assert.equal(await p.inputValue('#m-text'), 'Dani')
  assert.equal(await p.isChecked('#m-check'), false)
  assert.equal(await p.isChecked('#r-p'), true)
  assert.equal(await p.inputValue('#m-sel'), 'blue')
  assert.deepEqual(p.errors, [])
})

test('sync and async actions, ctx.call, submit and ctx.update', async () => {
  const p = await open('/directives')
  const reqs = []
  p.on('request', r => r.method() === 'POST' && reqs.push([new URL(r.url()).pathname, r.postData(), r.headers().traceparent?.length]))
  await p.click('#save')
  await p.waitForSelector('#saving', { state: 'visible' })
  await p.waitForSelector('#response')
  assert.equal(await text(p, '#response'), 'Ana')
  assert.equal(await p.isVisible('#saving'), false)
  assert.deepEqual(reqs[0], ['/_action/directives/save', '{"name":"Ana"}', 55])
  await p.click('#fail')
  await p.waitForFunction(() => document.querySelector('#error').textContent === 'Failed')
  await p.click('#submit')
  assert.equal(await text(p, '#sent'), 'true')
  assert.equal(new URL(p.url()).pathname, '/directives')
  await p.evaluate(() => setTimeout(() => __ctx.update(s => { s.tick = 42 }), 10))
  await p.waitForFunction(() => document.querySelector('#tick').textContent === '42')
})

test('persisted store updates layout and page and survives reload', async () => {
  const p = await open('/directives')
  assert.equal(await text(p, '#qty'), '0')
  await p.click('#buy')
  await p.click('#buy')
  assert.equal(await text(p, '#qty'), '2')
  assert.equal(await p.evaluate(() => localStorage.getItem('pragma:cart')), '{"items":[{"id":1},{"id":2}]}')
  await p.reload()
  await p.waitForFunction(() => document.querySelector('#qty').textContent === '2')
})

test('slot content belongs to whoever wrote it (lexical owner)', async () => {
  const p = await open('/slot')
  assert.equal(await text(p, '#box-title'), 'from box')
  assert.equal(await text(p, '#page-title'), 'from page')
  assert.equal(await text(p, '#page-if'), 'from page')
  await p.click('#page-btn')
  await p.click('#deep-btn')
  assert.equal(await p.evaluate(() => __slot.$.clicks), 2)
  assert.equal(await text(p, '#box-n'), '5')
  await p.click('#middle-btn')
  await p.click('#box-inc')
  assert.equal(await text(p, '#box-n'), '7')
  assert.equal(await text(p, '.count'), '0')
  await p.fill('#page-field', 'hi')
  assert.equal(await text(p, '#field'), 'hi')
  await p.locator('.item-btn').nth(1).click()
  assert.equal(await text(p, '#picked'), 'two')
  assert.deepEqual(p.errors, [])
})

test('props: data-prop-* coerced by the default type, data-with-* live from the caller', async () => {
  const p = await open('/slot')
  assert.deepEqual(await p.$$eval('li .n', e => e.map(x => x.textContent)), ['1', '1'])
  assert.deepEqual(await p.$$eval('li .max', e => e.map(x => x.textContent)), ['3', '7'])
  assert.equal(await text(p, '#box-n'), '5')
  // `export let initial = 0`: the text "5" becomes the number 5, so +1 gives 6, not "51"
  await p.click('#box-inc')
  assert.equal(await text(p, '#box-n'), '6')
  // a data-with prop follows the caller's value on the next render
  await p.evaluate(() => __slot.update(s => { s.items[1].max = 9 }))
  await p.waitForFunction(() => [...document.querySelectorAll('li .max')].map(x => x.textContent).join() === '3,9')
  assert.deepEqual(p.errors, [])
})

test('store getters are derived: recomputed and kept out of localStorage', async () => {
  const p = await open('/slot')
  assert.equal(await text(p, '#qty'), '10')
  assert.equal(await text(p, '#double'), '20')
  await p.click('#page-btn')
  assert.equal(await text(p, '#qty'), '20')
  assert.equal(await text(p, '#double'), '40')
  assert.equal(await p.evaluate(() => localStorage.getItem('pragma:account')), '{"n":2}')
  await p.reload()
  await p.waitForFunction(() => document.querySelector('#qty').textContent === '20')
  assert.deepEqual(p.errors, [])
})

test('router: aria-current="page" on links to the current route', async () => {
  const p = await open('/slot')
  const current = () => p.$$eval('a[aria-current="page"]', e => e.map(x => x.id))
  assert.deepEqual(await current(), ['nav-slot', 'link-slot'])
  await p.click('#nav-a')
  await p.waitForFunction(() => document.title === 'Page A')
  assert.deepEqual(await current(), ['nav-a'])
})

test('overlay.js is only fetched on pages with overlays', async () => {
  const p = await open('/directives')
  await tick(p)
  assert.equal(p.requests.some(r => r.endsWith('/overlay.js') && r.startsWith('/core/')), false)
  const o = await open('/overlay')
  await o.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/core/overlay.js')))
})

test('data-attr-checked sets the property and a new block with autofocus gets focus', async () => {
  const p = await open('/slot')
  await p.check('#chk')
  await p.evaluate(() => __slot.update(s => { s.marked = true }))
  await p.evaluate(() => __slot.update(s => { s.marked = false }))
  assert.equal(await p.isChecked('#chk'), false)
  await p.evaluate(() => __slot.update(s => { s.editing = true }))
  assert.equal(await p.evaluate(() => document.activeElement.id), 'edit')
})

test('script state is reactive: deep writes, array methods, one render per microtask, read-only props', async () => {
  const p = await open('/directives')
  await p.evaluate(() => { __ctx.$.person.name = 'Zé' })
  await p.waitForFunction(() => document.querySelector('#w').textContent === 'Zé')
  await p.evaluate(() => { __ctx.$.items.push({ id: 3, name: 'three', ok: true, tags: [] }) })
  await p.waitForFunction(() => document.querySelectorAll('#list li .n').length === 3)
  await p.evaluate(() => { __ctx.$.items[0].tags.splice(0, 1) })
  await p.waitForFunction(() => document.querySelectorAll('#list li:first-of-type .tag').length === 1)
  const batch = await p.evaluate(async () => {
    let records = 0
    const mo = new MutationObserver(rs => { records += rs.length })
    mo.observe(document.getElementById('name'), { childList: true, characterData: true, subtree: true })
    __ctx.$.name = 'a'
    __ctx.$.name = 'b'
    __ctx.$.name = 'c'
    await new Promise(r => setTimeout(r))
    mo.disconnect()
    return [records, document.getElementById('name').textContent]
  })
  assert.deepEqual(batch, [1, 'c'])
  const slot = await open('/slot')
  const error = await slot.evaluate(() => {
    try { __boxes[0].initial = 9; return 'written' } catch (e) { return e.message }
  })
  assert.match(error, /"initial" is a prop and props are read-only/)
  assert.equal(await slot.evaluate(() => __boxes[0].initial), 5)
  assert.deepEqual(p.errors, [])
})

test('unknown path throws in dev', async () => {
  const p = await open('/directives')
  await p.evaluate(() => __ctx.update(s => { delete s.name }))
  await tick(p)
  assert.match(p.errors.join(), /unknown path "state.name"/)
})

test('loader: skeleton, data, reused __pd and params', async () => {
  const p = await open('/product/7')
  assert.equal(await p.locator('#sk').count(), 1)
  assert.equal(await text(p, '#param'), '7')
  await p.waitForSelector('#name')
  assert.equal(await text(p, '#name'), 'Product 7')
  assert.equal(await p.locator('#sk').count(), 0)
  assert.equal(p.requests.filter(r => r === '/_data/product/7').length, 1)
})

test('loader: error becomes page.error', async () => {
  const p = await open('/error')
  await p.waitForSelector('#error')
  assert.equal(await text(p, '#error'), 'Load went wrong')
  assert.equal(await p.locator('#name').count(), 0)
})

test('SSR: adopts the DOM without recreating nodes and binds state', async () => {
  const p = await open('/ssr')
  const r = await p.evaluate(() => ({
    alive: __before.every(e => e.isConnected),
    mut: __mut,
    lis: [...document.querySelectorAll('#l li[data-of]')].map(l => l.textContent),
    d: document.querySelectorAll('#d').length,
    cd: document.querySelectorAll('#cd').length,
  }))
  assert.equal(r.alive, true)
  assert.deepEqual(r.mut, ['childList:c'])
  assert.deepEqual(r.lis, ['one', 'two'])
  assert.equal(r.d, 1)
  assert.equal(r.cd, 1)
  await p.click('#inc')
  assert.equal(await text(p, '#c'), '1')
  assert.equal(p.requests.some(u => u.startsWith('/_data')), false)
  assert.deepEqual(p.errors, [])
})

test('router: link swaps main and title, unmounts the page and does not reload', async () => {
  const p = await open('/a')
  await p.waitForFunction(() => self.__ticks > 2)
  await p.evaluate(() => { self.__mark = 1 })
  await p.evaluate(() => document.querySelector('#go-b').click())
  await p.waitForFunction(() => document.title === 'Page B')
  assert.equal(await text(p, '#title'), 'Page B')
  assert.equal(new URL(p.url()).pathname, '/b')
  assert.equal(await p.evaluate(() => self.__mark), 1)
  assert.equal(await p.evaluate(() => self.__cleaned), true)
  const t = await p.evaluate(() => self.__ticks)
  await p.waitForTimeout(80)
  assert.equal(await p.evaluate(() => self.__ticks), t)
  assert.equal(await p.evaluate(() => document.activeElement.matches('main[data-page]')), true)
  await p.evaluate(() => document.querySelector('#d').click())
  await p.waitForSelector('#list li')
  assert.equal(await text(p, '#name'), 'Ana')
  assert.equal(await p.evaluate(() => self.__mark), 1)
  assert.deepEqual(p.errors, [])
})

test('router: static beats param and loader on navigation', async () => {
  const p = await open('/a')
  await p.evaluate(() => document.querySelector('#new').click())
  await p.waitForFunction(() => document.title === 'New product')
  await p.goBack()
  await p.waitForFunction(() => document.title === 'Page A')
  await p.evaluate(() => document.querySelector('#p1').click())
  await p.waitForSelector('#sk')
  await p.waitForSelector('#name')
  assert.equal(await text(p, '#name'), 'Product 1')
  assert.equal(await text(p, '#param'), '1')
})

test('router: prefetch on hover and by viewport', async () => {
  const p = await open('/a')
  await p.evaluate(() => document.querySelector('#p1').dispatchEvent(new PointerEvent('pointerenter')))
  await p.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.includes('/product/%5Bid%5D.html') || e.name.includes('/product/[id].html')))
  const b = await open('/b')
  await b.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/assets/ssr.js')))
  await b.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/directives.html')))
  // a link in view downloads the route module but does not run it until navigation
  assert.equal(await b.evaluate(() => !!document.querySelector('link[rel=modulepreload][href$="/assets/ssr.js"]')), true)
  assert.equal(await b.evaluate(() => self.__ssrModuleRan), undefined)
  assert.equal(b.requests.includes('/ssr'), false)
  assert.equal(b.requests.includes('/back'), false)
  const before = b.requests.filter(r => r === '/directives.html').length
  await b.evaluate(() => document.querySelector('#d').click())
  await b.waitForSelector('#list li')
  assert.equal(b.requests.filter(r => r === '/directives.html').length, before)
  await b.goBack()
  await b.waitForFunction(() => document.title === 'Page B')
  await b.evaluate(() => document.querySelector('#s').click())
  await b.waitForFunction(() => self.__ssrModuleRan === true && document.querySelector('#c'))
  assert.deepEqual(b.errors, [])
})

test('router: view transition only without prefers-reduced-motion', async () => {
  const p = await open('/b')
  await p.evaluate(() => {
    self.__transitions = 0
    const start = document.startViewTransition.bind(document)
    document.startViewTransition = swap => (self.__transitions++, start(swap))
  })
  await p.emulateMedia({ reducedMotion: 'reduce' })
  await p.evaluate(() => document.querySelector('#back').click())
  await p.waitForFunction(() => document.title === 'Page A')
  assert.equal(await p.evaluate(() => self.__transitions), 0)
  await p.emulateMedia({ reducedMotion: 'no-preference' })
  await p.evaluate(() => document.querySelector('#go-b').click())
  await p.waitForFunction(() => document.title === 'Page B')
  assert.equal(await p.evaluate(() => self.__transitions), 1)
  assert.deepEqual(p.errors, [])
})

test('router: back and forward restore scroll', async () => {
  const p = await open('/a')
  await p.evaluate(() => scrollTo(0, 1200))
  await p.evaluate(() => document.querySelector('#go-b').click())
  await p.waitForFunction(() => document.title === 'Page B')
  assert.equal(await p.evaluate(() => scrollY), 0)
  await p.evaluate(() => scrollTo(0, 700))
  await p.goBack()
  await p.waitForFunction(() => document.title === 'Page A')
  await p.waitForFunction(() => scrollY === 1200)
  await p.goForward()
  await p.waitForFunction(() => document.title === 'Page B')
  await p.waitForFunction(() => scrollY === 700)
  assert.deepEqual(p.errors, [])
})

test('router: popstate with an open overlay is consumed by the overlay', async () => {
  const p = await open('/overlay')
  await p.evaluate(() => { self.__mark = 1 })
  await p.click('#open')
  assert.equal(await p.evaluate(() => document.querySelector('#m').open), true)
  await p.evaluate(() => history.back())
  await p.waitForFunction(() => !document.querySelector('#m').open)
  await p.waitForTimeout(50)
  assert.equal(new URL(p.url()).pathname, '/overlay')
  assert.equal(await p.evaluate(() => self.__mark), 1)
  assert.equal(await text(p, '#title'), 'Overlay')
})

test('performance: 1000 items with keyed diff', async () => {
  const p = await open('/list')
  const r = await p.evaluate(async () => {
    const flush = () => new Promise(r => queueMicrotask(r))
    const time = async fn => {
      const t = performance.now()
      __ctx.update(fn)
      await flush()
      return performance.now() - t
    }
    const before = [...document.querySelectorAll('#l li')]
    const items = __ctx.$.items
    const out = {}
    out.empty = await time(s => { s.items = [] })
    out.create1000 = await time(s => { s.items = items })
    const created = [...document.querySelectorAll('#l li')]
    const one = []
    for (let i = 0; i < 20; i++) one.push(await time(s => { s.items[500].name = 'changed ' + i; s.items[500].done = i % 2 === 0 }))
    out.change1 = one.sort((a, b) => a - b)[10]
    out.swap2 = await time(s => { const x = s.items[1]; s.items[1] = s.items[998]; s.items[998] = x })
    out.remove1 = await time(s => { s.items.splice(10, 1) })
    const after = [...document.querySelectorAll('#l li')]
    out.preserved = after.every(li => created.includes(li))
    out.n = after.length
    out.text = document.querySelectorAll('#l li')[499].textContent
    out.ssrBefore = before.length
    return out
  })
  assert.equal(r.preserved, true)
  assert.equal(r.n, 999)
  assert.equal(r.text, 'changed 19')
  console.log(`performance (ms): create 1000 = ${r.create1000.toFixed(2)}, change 1 of 1000 (median) = ${r.change1.toFixed(2)}, swap 2 = ${r.swap2.toFixed(2)}, remove 1 = ${r.remove1.toFixed(2)}, clear = ${r.empty.toFixed(2)}`)
})

test('{} text: element templates, comment markers, attribute templates and derived functions', async () => {
  const p = await open('/template')
  assert.equal(await text(p, '#hello'), 'Hello Ana, you have 2 items {literal} & more')
  assert.deepEqual([await p.getAttribute('#link', 'href'), await p.getAttribute('#link', 'title')], ['/u/7?tab=1', 'Open Ana'])
  assert.equal(await text(p, '#mixed'), 'Total: 2 tasks, Ana!')
  assert.equal(await text(p, '#pending'), '1 pending')
  assert.deepEqual(await p.$$eval('#tasks label', ls => ls.map(l => l.textContent)), [' Buy <milk>', ' Walk'])
  assert.equal(await p.locator('#tasks milk').count(), 0)
  const adopted = await p.evaluate(() => {
    const el = document.querySelector('#adopt')
    return [el.textContent, el.childNodes.length]
  })
  assert.deepEqual(adopted, ['Hi Ana!', 5])
  assert.equal(await text(p, '#code'), '{literal}: Buy <milk>;Walk;')
  assert.deepEqual(await p.$$eval('#code span', s => s.map(x => x.dataset.n)), ['1', '2'])
  assert.equal(await text(p, '[data-c="inner"] .own'), 'Inner')
  assert.equal(await p.evaluate(() => document.querySelector('[data-c="inner"]').textContent), 'Inner Ana')

  const writes = await p.evaluate(async () => {
    const seen = []
    const mo = new MutationObserver(rs => seen.push(...rs))
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true })
    __page.count = 2
    await new Promise(r => setTimeout(r))
    mo.disconnect()
    return seen.length
  })
  assert.equal(writes, 0)

  await p.evaluate(() => {
    __page.name = 'Bo'
    __page.tasks.push({ id: 3, title: 'Read', done: false })
  })
  await tick(p)
  assert.equal(await text(p, '#hello'), 'Hello Bo, you have 2 items {literal} & more')
  assert.equal(await text(p, '#mixed'), 'Total: 3 tasks, Bo!')
  assert.equal(await text(p, '#pending'), '2 pending')
  assert.equal(await p.getAttribute('#link', 'title'), 'Open Bo')
  assert.equal(await p.evaluate(() => document.querySelector('[data-c="inner"]').textContent), 'Inner Bo')
  assert.deepEqual(p.errors, [])
})

test('events call functions with arguments: paths, literals and event', async () => {
  const p = await open('/template')
  await p.locator('#tasks li[data-key="1"] input').click()
  assert.equal(await text(p, '#pending'), '0 pending')
  assert.equal(await p.getAttribute('#tasks li[data-key="1"]', 'class'), 'done')
  await p.locator('#tasks li[data-key="2"] .rename').click()
  assert.deepEqual(await p.$$eval('#tasks label', ls => ls.map(l => l.textContent)), [' Buy <milk>', ' Renamed'])
  await p.click('#record')
  assert.equal(await text(p, '#log'), 'click|a, b|-2.5|true||Ana')
  assert.deepEqual(p.errors, [])
})
