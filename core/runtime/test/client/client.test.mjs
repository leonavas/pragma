import { test, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { start } from './server.mjs'

const tools = process.env.PRAGMA_TOOLS ?? `${process.env.HOME}/.cache/pragma-tools`
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

test('<if>/<else>, data-with and writes only on change', async () => {
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

test('nested <for>/<empty> and keyed diff preserving nodes', async () => {
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
  // dev: an object prop is read-only inside too (it is the caller's data); copies are free to change
  const deep = await slot.evaluate(() => {
    const box = __boxes.find(b => b.row)
    const errors = []
    for (const write of [() => { box.row.max = 99 }, () => { delete box.row.name }, () => { Object.assign(box.row, { id: 5 }) }]) {
      try { write(); errors.push('written') } catch (e) { errors.push(e.message) }
    }
    const copy = { ...box.row }
    copy.max = 99
    return { errors, max: box.row.max, copy: copy.max, array: Array.isArray(__slot.$.items) }
  })
  assert.ok(deep.errors.every(m => /"row" is a prop: props are read-only, also inside/.test(m)), deep.errors.join(' | '))
  assert.deepEqual([deep.max, deep.copy, deep.array], [3, 99, true])
  assert.deepEqual(p.errors, [])
})

test('async mount: the resolved function is the cleanup', async () => {
  const p = await open('/a')
  await p.waitForFunction(() => self.__ticks > 2)
  await p.evaluate(() => document.querySelector('#go-b').click())
  await p.waitForFunction(() => document.title === 'Page B')
  assert.equal(await p.evaluate(() => self.__cleaned), true, 'the cleanup the async mount resolved to ran')
  assert.deepEqual(p.errors, [])
})

test('unknown path throws in dev', async () => {
  const p = await open('/directives')
  await p.evaluate(() => __ctx.update(s => { delete s.name }))
  await tick(p)
  assert.match(p.errors.join(), /unknown path "state.name"/)
})

test('null in the middle of a path renders empty in dev, as on the server', async () => {
  const p = await open('/directives')
  await p.evaluate(() => __ctx.update(s => { s.person = null }))
  await p.waitForFunction(() => document.querySelector('#w').textContent === '')
  assert.deepEqual(p.errors, [])
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
  assert.equal(await p.evaluate(() => document.activeElement.matches('main[data-page]')), true)
  assert.equal(await p.evaluate(() => document.querySelector('main').dataset.page), 'b', 'the page CSS scope follows the page')
  await p.evaluate(() => document.querySelector('#d').click())
  await p.waitForSelector('#list li')
  assert.equal(await p.evaluate(() => document.querySelector('main').dataset.page), '')
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
  await p.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/product/%5Bid%5D')))
  const b = await open('/b')
  await b.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/assets/ssr.js')))
  await b.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/directives')))
  // a link in view downloads the route module but does not run it until navigation
  assert.equal(await b.evaluate(() => !!document.querySelector('link[rel=modulepreload][href$="/assets/ssr.js"]')), true)
  assert.equal(await b.evaluate(() => self.__ssrModuleRan), undefined)
  assert.equal(b.requests.includes('/ssr'), false)
  assert.equal(b.requests.includes('/back'), false)
  assert.equal(b.requests.some(r => r.endsWith('.html')), false)
  const before = b.requests.filter(r => r === '/directives').length
  await b.evaluate(() => document.querySelector('#d').click())
  await b.waitForSelector('#list li')
  assert.equal(b.requests.filter(r => r === '/directives').length, before)
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

test('performance: 60 frames of a :--x value on a 1000-item page patch only what reads x', async () => {
  const p = await open('/fast')
  const r = await p.evaluate(async () => {
    const flush = () => new Promise(r => queueMicrotask(r))
    const frame = () => new Promise(r => requestAnimationFrame(r))
    const seen = []
    const mo = new MutationObserver(rs => seen.push(...rs))
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true })
    const rows = self.__rows, times = []
    for (let i = 1; i <= 60; i++) {
      await frame()
      const t = performance.now()
      __ctx.$.x = i % 30
      await flush()
      times.push(performance.now() - t)
    }
    await flush()
    mo.disconnect()
    return { rows: self.__rows - rows, ops: seen.length, total: times.reduce((a, b) => a + b, 0), median: times.sort((a, b) => a - b)[30] }
  })
  console.log(`performance (ms): 60 :--x updates on 1000 items = ${r.total.toFixed(2)} total, ${r.median.toFixed(3)} median; DOM writes = ${r.ops}`)
  assert.equal(r.rows, 0)
  assert.equal(r.ops, 120)
  assert.equal(await p.$eval('#dot', el => [el.style.getPropertyValue('--x'), el.getAttribute('aria-valuenow')].join()), '0,0')
  assert.deepEqual(p.errors, [])
})

test('binding-only changes patch the same DOM a full render would', async () => {
  const p = await open('/fast')
  const view = () => p.evaluate(() => ({
    dot: [document.querySelector('#dot').style.getPropertyValue('--x'), document.querySelector('#dot').className, document.querySelector('#dot').getAttribute('aria-valuenow')],
    label: document.querySelector('#label').textContent,
    open: document.querySelector('#open')?.textContent ?? null,
    first: document.querySelector('#l li').className + ':' + document.querySelector('#l li').textContent,
    n: document.querySelectorAll('#l li').length,
    rows: self.__rows,
  }))
  const step = fn => p.evaluate(async fn => { new Function('s', fn)(__ctx.$); await new Promise(r => setTimeout(r)) }, fn)
  const rows = (await view()).rows

  await step('s.x = 40')
  assert.deepEqual(await view(), { dot: ['40', 'far', '40'], label: 'far a', open: null, first: 'done:item 0', n: 1000, rows })
  await step("s.y = 'b'")
  assert.equal((await view()).label, 'far b')
  await step("s.items[0].name = 'zero'; s.items[0].done = false")
  assert.deepEqual(await view(), { dot: ['40', 'far', '40'], label: 'far b', open: null, first: ':zero', n: 1000, rows })

  await step('s.open = true; s.x = 5')
  assert.deepEqual(await view(), { dot: ['5', '', '5'], label: 'near', open: 'x = 5', first: ':zero', n: 1000, rows: rows + 1 })
  await step('s.x = 6')
  assert.equal((await view()).open, 'x = 6')
  await step("s.items.push({ id: 1000, name: 'new', done: true })")
  assert.deepEqual([(await view()).n, (await view()).rows], [1001, rows + 2])
  await step('s.x = 7; s.open = false')
  assert.deepEqual([(await view()).open, (await view()).dot[0], (await view()).rows], [null, '7', rows + 3])
  assert.deepEqual(p.errors, [])
})

test('literal props: false, numbers and strings reach the instance typed', async () => {
  const p = await open('/frame')
  assert.equal(await text(p, '#flag span'), 'false|-2.5|a b|Ana')
  await p.evaluate(() => { __ctx.$.who = 'Bia' })
  await tick(p)
  assert.equal(await text(p, '#flag span'), 'false|-2.5|a b|Bia')
  assert.deepEqual(p.errors, [])
})

test('performance: 10 layout reads and state changes per frame on a 1000-item page, requestAnimationFrame vs frame()', async () => {
  const p = await open('/frame')
  const run = schedule => p.evaluate(async schedule => {
    const box = document.querySelector('#box'), frames = 30, writers = 10, times = []
    let writes = 0
    const mo = new MutationObserver(rs => { writes += rs.length })
    mo.observe(box, { attributes: true, attributeFilter: ['style'] })
    const later = schedule === 'frame' ? __ctx.frame : requestAnimationFrame
    for (let f = 0; f < frames; f++) {
      await new Promise(done => {
        let start = 0, left = writers
        for (let w = 0; w < writers; w++) later(() => {
          start ||= performance.now()
          box.offsetHeight
          __ctx.$.x = ((f * writers + w) % 40) + 1
          if (--left) return
          queueMicrotask(() => {
            box.offsetHeight
            times.push(performance.now() - start)
            done()
          })
        })
      })
    }
    await new Promise(r => setTimeout(r))
    mo.disconnect()
    times.sort((a, b) => a - b)
    return { median: times[frames >> 1], writes: writes / frames }
  }, schedule)
  const raf = await run('raf'), batched = await run('frame')
  console.log(`performance (ms per frame, 10 read+write callbacks, 1000 items): requestAnimationFrame = ${raf.median.toFixed(2)} (${raf.writes} style writes), frame() = ${batched.median.toFixed(2)} (${batched.writes} style write)`)
  assert.equal(raf.writes, 10)
  assert.equal(batched.writes, 1)
  assert.equal(await p.$eval('#box', el => el.style.getPropertyValue('--x')), String(((29 * 10 + 9) % 40) + 1))
  assert.deepEqual(p.errors, [])
})

test('frame(): callbacks share one animation frame, run in order, and can be cancelled', async () => {
  const p = await open('/frame')
  const r = await p.evaluate(() => new Promise(done => {
    const seen = []
    __ctx.frame(now => seen.push(['a', typeof now]))
    const cancel = __ctx.frame(() => seen.push(['b']))
    __ctx.frame(() => {
      seen.push(['c'])
      __ctx.frame(() => done(seen.concat([['next']])))
    })
    cancel()
  }))
  assert.deepEqual(r, [['a', 'number'], ['c'], ['next']])
  assert.deepEqual(p.errors, [])
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
  await p.fill('#search', 'mug')
  assert.equal(await text(p, '#query'), 'mug')
  await p.click('#locate b')
  assert.equal(await text(p, '#spot'), 'locate:click')
  assert.deepEqual(await p.$$eval('#basket .drop', b => b.map(x => x.textContent)), ['Pen', 'Ink'])
  await p.locator('#basket .drop', { hasText: 'Pen' }).click()
  assert.deepEqual(await p.$$eval('#basket .drop', b => b.map(x => x.textContent)), ['Ink'])
  assert.deepEqual(p.errors, [])
})

test('a const holding a function made by a call is a handler; a non-function throws in dev', async () => {
  const p = await open('/template')
  await p.click('#save-twice')
  assert.equal(await text(p, '#saves'), '2')
  assert.deepEqual(p.errors, [])
  await p.click('#not-a-function')
  assert.match(p.errors.join(), /@click="name" needs a function, but "name" holds string/)
})

test('loader: a redirect replaces the history entry and an error keeps its status and body', async () => {
  const p = await open('/gone')
  await p.waitForSelector('#title')
  assert.equal(new URL(p.url()).pathname, '/b')
  assert.equal(await p.evaluate(() => history.length), 2)
  await p.goBack()
  assert.equal(new URL(p.url()).href, 'about:blank')
  assert.equal(await p.locator('#gone').count(), 0)

  const m = await open('/missing')
  await m.waitForSelector('#error')
  assert.deepEqual([await text(m, '#status'), await text(m, '#code'), await text(m, '#message')], ['404', 'missing', 'No product'])
  assert.deepEqual(p.errors, [])
})

test('call: errors carry status and body; a redirect navigates and resolves to undefined', async () => {
  const p = await open('/net')
  const failures = await p.evaluate(async () => {
    const out = []
    for (const name of ['conflict', 'plain']) {
      try { await __net.call(name, { day: 1 }) } catch (e) { out.push([e instanceof Error, e.name, e.status, e.body, e.message]) }
    }
    return out
  })
  assert.deepEqual(failures, [
    [true, 'ActionError', 409, { message: 'Slot taken', field: 'day' }, 'Slot taken'],
    [true, 'ActionError', 400, 'bad input', 'bad input'],
  ])
  const after = await p.evaluate(async () => {
    const before = history.length, r = await __net.call('go')
    return [r, location.pathname, document.querySelector('#title').textContent, history.length - before]
  })
  assert.deepEqual(after, [undefined, '/b', 'Page B', 1])
  assert.deepEqual(p.errors, [])
})

test('router: a page fetch redirected by middleware goes to the final URL; a .html to clean URL redirect stays', async () => {
  const p = await open('/net')
  const before = await p.evaluate(() => history.length)
  await p.evaluate(() => __net.navigate('/private'))
  await p.waitForFunction(() => location.pathname === '/b' && document.querySelector('#title')?.textContent === 'Page B')
  assert.equal(await p.evaluate(() => history.length) - before, 1)
  const tidy = await p.evaluate(async () => {
    await __net.navigate('/tidy')
    return [location.pathname, document.querySelector('#title')?.textContent]
  })
  assert.deepEqual(tidy, ['/tidy', 'Page B'])
  assert.deepEqual(p.errors, [])
})

test('navigate with replace uses replaceState', async () => {
  const p = await open('/net')
  const r = await p.evaluate(async () => {
    const before = history.length
    await __net.navigate('/b', { replace: true })
    return [location.pathname, history.length - before]
  })
  assert.deepEqual(r, ['/b', 0])
})

test('stream: yields NDJSON lines as they arrive, aborts, throws ActionError and follows a JSON redirect', async () => {
  const p = await open('/net')
  const chat = await p.evaluate(async () => {
    const t0 = performance.now(), out = []
    for await (const line of __net.stream('chat', { q: 1 })) out.push([line, performance.now() - t0])
    return out
  })
  assert.deepEqual(chat.map(c => c[0]), [{ accept: 'application/x-ndjson, application/json' }, { n: 1 }, { n: 2, s: 'ã' }, { n: 3 }])
  assert.ok(chat[3][1] - chat[0][1] > 200, 'lines arrive while the response streams')

  const aborted = await p.evaluate(async () => {
    const c = new AbortController(), out = []
    try {
      for await (const line of __net.stream('slow', null, { signal: c.signal })) {
        out.push(line.n)
        if (out.length === 2) c.abort()
      }
    } catch (e) { return [out, e.name] }
    return [out, 'no error']
  })
  assert.deepEqual(aborted, [[0, 1], 'AbortError'])

  const failed = await p.evaluate(async () => {
    try { for await (const _ of __net.stream('conflict')) {} } catch (e) { return [e.name, e.status, e.body.field] }
  })
  assert.deepEqual(failed, ['ActionError', 409, 'day'])

  const redirected = await p.evaluate(async () => {
    const out = []
    for await (const line of __net.stream('go')) out.push(line)
    return [out, location.pathname]
  })
  assert.deepEqual(redirected, [[], '/b'])
  assert.deepEqual(p.errors, [])
})

const motion = p => p.evaluate(() => [...document.querySelectorAll('main p, #l li')].map(n => n.textContent + (n.hasAttribute('data-leaving') ? ':leaving' : '') + (n.hasAttribute('data-entering') ? ':entering' : '') + (n.inert ? ':inert' : '')))

test('motion: data-leaving keeps the node until its CSS animation ends; data-entering only after the first render', async () => {
  const p = await open('/motion')
  assert.deepEqual(await motion(p), ['on', 'a', 'b', 'c', 'plain'])

  await p.evaluate(() => {
    self.__on = document.querySelector('#on')
    self.__li = [...document.querySelectorAll('#l li')]
    __ctx.update(s => {
      s.on = false
      s.items = ['a', 'c', 'd']
      s.plain = false
    })
  })
  await tick(p)
  assert.deepEqual(await motion(p), ['on:leaving:inert', 'off:entering', 'a', 'b:leaving:inert', 'c', 'd:entering'])
  assert.deepEqual(await p.evaluate(() => { const l = [...document.querySelectorAll('#l li')]; return [l[0] === __li[0], l[2] === __li[2]] }), [true, true])

  await p.evaluate(() => __ctx.update(s => { s.on = true }))
  await tick(p)
  assert.deepEqual(await motion(p), ['on', 'off:leaving:inert', 'a', 'b:leaving:inert', 'c', 'd:entering'])
  assert.equal(await p.evaluate(() => document.querySelector('#on') === __on), true)

  await p.waitForFunction(() => !document.querySelector('[data-leaving], [data-entering]'))
  assert.deepEqual(await motion(p), ['on', 'a', 'c', 'd'])
  assert.deepEqual(p.errors, [])
})

test('motion: removal is immediate under prefers-reduced-motion', async () => {
  const ctx = await browser.newContext({ reducedMotion: 'reduce' })
  const p = await ctx.newPage()
  await p.goto(server.url + '/motion')
  await p.waitForFunction(() => self.__ctx)
  await p.evaluate(() => __ctx.update(s => { s.on = false; s.items = ['a', 'x'] }))
  await tick(p)
  assert.deepEqual(await motion(p), ['off', 'a', 'x', 'plain'])
})

test(':--name sets CSS custom properties: numbers as is, false/null removes, writes only on change', async () => {
  const p = await open('/tree')
  const vars = (sel, ...names) => p.$eval(sel, (el, names) => names.map(n => el.style.getPropertyValue(n)), names)
  assert.deepEqual(await vars('#bar', '--fixed', '--left', '--tone'), ['1', '25', 'red'])
  assert.deepEqual(await p.$$eval('#rows li', ls => ls.map(l => l.style.getPropertyValue('--x'))), ['10', ''])
  const writes = await p.evaluate(async () => {
    const seen = []
    const mo = new MutationObserver(rs => seen.push(...rs))
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true })
    __page.count = 1
    await new Promise(r => setTimeout(r))
    mo.disconnect()
    return seen.length
  })
  assert.equal(writes, 0)
  await p.evaluate(() => {
    __page.left = 12.5
    __page.tone = false
    __page.rows[1].x = 0
  })
  await tick(p)
  assert.deepEqual(await vars('#bar', '--fixed', '--left', '--tone'), ['1', '12.5', ''])
  assert.deepEqual(await p.$$eval('#rows li', ls => ls.map(l => l.style.getPropertyValue('--x'))), ['10', '0'])
  assert.deepEqual(p.errors, [])
  await p.evaluate(() => { __page.left = '1;color:red' })
  await tick(p)
  assert.equal(await p.$eval('#bar', el => el.style.color), '')
  assert.match(p.errors.join(' | '), /:--left got "1;color:red": a CSS value cannot contain ; \{ \} or </)
})

test('a component renders itself: tree from data, adopts prerendered levels, depth guard', async () => {
  const p = await open('/tree')
  const names = () => p.$$eval('#tree span', s => s.map(x => x.textContent))
  assert.deepEqual(await names(), ['root', 'A', 'A1', 'B'])
  assert.deepEqual(await p.$$eval('#tree li[data-self]', ls => ls.map(l => l.style.getPropertyValue('--depth'))), ['1', '2', '1'])
  assert.equal(await p.locator('#adopted').textContent(), 'A')
  assert.equal(await p.locator('#tree li[data-key="a"] > span').count(), 1)
  assert.deepEqual(await p.$$eval('#dots circle', cs => cs.map(c => [c.namespaceURI, c.getAttribute('cx')])), [['http://www.w3.org/2000/svg', '1'], ['http://www.w3.org/2000/svg', '2']], 'a body inside <svg> is parsed as SVG')

  await p.evaluate(() => {
    __page.root.children[1].children.push({ id: 'b1', name: 'B1', depth: 2, children: [{ id: 'b2', name: 'B2', depth: 3, children: [] }] })
    __page.root.children[0].name = 'A!'
  })
  await tick(p)
  assert.deepEqual(await names(), ['root', 'A!', 'A1', 'B', 'B1', 'B2'])
  assert.equal(await p.locator('#adopted').textContent(), 'A!')
  await p.evaluate(() => { __page.root.children.shift() })
  await tick(p)
  assert.deepEqual(await names(), ['root', 'B', 'B1', 'B2'])
  assert.deepEqual(await p.$eval('#tree', el => {
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_COMMENT), anchors = []
    while (walker.nextNode()) anchors.push(walker.currentNode.data)
    return [anchors, el.querySelectorAll('template').length]
  }), [Array(4).fill('p:for t9 child state.node.children child.id'), 0], 'items clone comment anchors; bodies come from the template module')
  assert.deepEqual(p.errors, [])

  await p.evaluate(() => document.querySelector('#go-b').click())
  await p.waitForFunction(() => document.title === 'Page B')
  await p.goBack()
  await p.waitForFunction(() => document.querySelector('#tree span')?.textContent === 'root')
  await p.evaluate(() => { __page.root.children[1].children.push({ id: 'b1', name: 'B1', depth: 2, children: [] }) })
  await tick(p)
  assert.deepEqual(await names(), ['root', 'A', 'A1', 'B', 'B1'], 'after a page swap the new page templates are used')
  assert.deepEqual(p.errors, [])

  await p.evaluate(() => {
    let node = { id: 'n0', name: 'n0', depth: 0, children: [] }
    const top = node
    for (let i = 1; i <= 70; i++) node.children.push((node = { id: 'n' + i, name: 'n' + i, depth: i, children: [] }))
    __page.root = top
  })
  await tick(p)
  assert.match(p.errors.join(' | '), /<c-node> is nested more than 64 levels inside itself/)
})

test('router: a link picks the same route as a full load (static segment first, as on the server)', async () => {
  const p = await open('/net')
  await p.evaluate(() => __net.navigate('/docs/y/z'))
  await p.waitForFunction(() => location.pathname === '/docs/y/z' && document.querySelector('#title')?.textContent !== 'Net')
  assert.equal(await text(p, '#title'), 'Docs')
  const full = await open('/docs/y/z')
  assert.equal(await text(full, '#title'), 'Docs')
  assert.deepEqual(p.errors, [])
})

test('an overlay that exists only inside a block opens', async () => {
  const p = await open('/menu')
  await p.click('#open-m2')
  await p.waitForFunction(() => document.querySelector('#m2').open)
  assert.deepEqual(p.errors, [])
})

test('a store named theme keeps its own key: the theme value is not read as store data, old store data moves', async () => {
  const visit = async init => {
    const ctx = await browser.newContext()
    await ctx.addInitScript(init)
    const page = await ctx.newPage()
    page.errors = []
    page.on('pageerror', e => page.errors.push(e.message))
    await page.goto(server.url + '/prefs')
    await page.waitForFunction(() => document.querySelector('#mode').textContent !== '')
    return page
  }
  const themed = await visit(() => localStorage.getItem('pragma:theme') ?? localStorage.setItem('pragma:theme', 'dark'))
  assert.equal(await text(themed, '#mode'), 'narrow')
  await themed.click('#wide')
  await themed.waitForFunction(() => localStorage.getItem('pragma:store:theme') === '{"mode":"wide"}')
  assert.equal(await themed.evaluate(() => localStorage.getItem('pragma:theme')), 'dark')

  const old = await visit(() => localStorage.getItem('pragma:store:theme') ?? localStorage.setItem('pragma:theme', '{"mode":"wide"}'))
  assert.equal(await text(old, '#mode'), 'wide')
  assert.deepEqual(await old.evaluate(() => [localStorage.getItem('pragma:theme'), localStorage.getItem('pragma:store:theme')]), [null, '{"mode":"wide"}'])
  assert.deepEqual([...themed.errors, ...old.errors], [])
})

test('a loop variable named like an Object.prototype member is passed to the handler', async () => {
  const p = await open('/prefs')
  await p.click('#picks button')
  assert.equal(await text(p, '#got'), 'lid')
  assert.deepEqual(p.errors, [])
})

test('event arguments accept the number literals the compiler accepts', async () => {
  const p = await open('/prefs')
  await p.click('#nums')
  assert.equal(await text(p, '#got'), 'number:0.5|number:1000|number:-0.0025|number:1|number:1|number:7')
  assert.deepEqual(p.errors, [])
})
