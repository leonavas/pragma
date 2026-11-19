import { describe, it, before, after, beforeEach, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { serve } from './server.mjs'

const pw = await import(process.env.PLAYWRIGHT ?? '/tmp/pragma-tools-overlays/node_modules/playwright/index.mjs')

const open = id => document.getElementById(id).matches('[open], :popover-open')

for (const name of ['chromium', 'firefox']) {
  describe(name, () => {
    let server, browser, page, base

    before(async () => {
      server = await serve()
      base = `http://127.0.0.1:${server.address().port}`
      // Classic scrollbar (headless hides it) to measure the scroll lock layout shift.
      browser = await pw[name].launch(name == 'chromium' ? { ignoreDefaultArgs: ['--hide-scrollbars'] } : {})
    })
    after(async () => {
      await browser.close()
      server.close()
    })
    afterEach(() => page.close())
    beforeEach(async () => {
      page = await browser.newPage()
      await page.goto(base + '/home.html')
      await page.click('#go')
      await page.waitForFunction(() => self.t)
      await page.evaluate(() => {
        self.pops = 0
        addEventListener('popstate', () => self.pops++)
      })
    })

    const state = () => page.evaluate(() => ({
      path: location.pathname,
      navigations: self.navigations,
      lock: document.documentElement.hasAttribute('data-scroll-lock'),
      focus: document.activeElement?.id,
    }))
    const waitOpen = (id, v = true) => page.waitForFunction(([id, v]) =>
      document.getElementById(id).matches('[open], :popover-open') == v, [id, v])
    const waitLock = v => page.waitForFunction(v => document.documentElement.hasAttribute('data-scroll-lock') == v, v)
    const waitPops = n => page.waitForFunction(n => self.pops == n, n)
    const backToStart = async () => {
      await page.goBack()
      await page.waitForURL(/home\.html$/)
    }

    it('opens and closes via button, without navigating and consuming the entry', async () => {
      await page.click('#open-about')
      assert.equal(await page.evaluate(open, 'about'), true)
      assert.equal((await state()).lock, true)
      await page.click('#ok-about')
      await waitOpen('about', false)
      await waitPops(1)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-about' })
      await backToStart()
    })

    it('Esc closes the modal', async () => {
      await page.click('#open-about')
      await page.keyboard.press('Escape')
      await waitOpen('about', false)
      await waitPops(1)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-about' })
      await backToStart()
    })

    it('backdrop click closes the modal but not the alert', async () => {
      await page.click('#open-about')
      await page.mouse.click(5, 5)
      await waitOpen('about', false)
      await waitPops(1)
      await page.click('#open-delete')
      await page.mouse.click(5, 5)
      await page.waitForTimeout(100)
      assert.equal(await page.evaluate(open, 'delete'), true)
      await page.click('#cancel-delete')
      await waitOpen('delete', false)
      await waitPops(2)
      assert.equal((await state()).navigations, 0)
      await backToStart()
    })

    it('back closes the overlay without changing the URL', async () => {
      await page.click('#open-about')
      await page.goBack()
      await waitOpen('about', false)
      await waitLock(false)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-about' })
      await backToStart()
    })

    it('a stack of two overlays closes top-down', async () => {
      await page.click('#open-about')
      await page.click('#open-details')
      assert.equal(await page.evaluate(open, 'details'), true)
      await page.keyboard.press('Escape')
      await waitOpen('details', false)
      await waitPops(1)
      assert.equal(await page.evaluate(open, 'about'), true)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: true, focus: 'open-details' })
      await page.goBack()
      await waitOpen('about', false)
      await waitLock(false)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-about' })
      await backToStart()
    })

    it('scroll lock locks and releases', async () => {
      await page.evaluate(() => scrollTo(0, 500))
      await page.evaluate(() => t.open('about'))
      const locked = await page.evaluate(() => getComputedStyle(document.documentElement).overflow)
      assert.equal(locked, 'hidden')
      await page.mouse.wheel(0, 800)
      await page.waitForTimeout(100)
      assert.equal(await page.evaluate(() => scrollY), 500)
      await page.click('#ok-about')
      await waitLock(false)
      assert.equal(await page.evaluate(() => scrollY), 500)
      assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).overflow), 'visible')
      await page.click('#open-more')
      assert.equal((await state()).lock, false)
    })

    for (const tall of [true, false]) {
      it(`scroll lock does not shift the layout (${tall ? 'with' : 'without'} scrollbar)`, async () => {
        await page.evaluate(tall => document.querySelector('main').classList.toggle('tall', tall), tall)
        const width = () => page.evaluate(() => document.body.getBoundingClientRect().width)
        const before = await width()
        await page.evaluate(() => t.open('about'))
        await waitLock(true)
        assert.deepEqual(await width(), before)
        await page.click('#ok-about')
        await waitLock(false)
        assert.deepEqual(await width(), before)
      })
    }

    it('focus returns to the opener, also via open()', async () => {
      await page.focus('#outside')
      await page.evaluate(() => t.open('about'))
      await page.evaluate(() => t.close('about'))
      await waitOpen('about', false)
      await waitPops(1)
      assert.equal((await state()).focus, 'outside')
      await backToStart()
    })

    it('confirm() resolves with each button value or cancel', async () => {
      const cases = [
        ['delete', () => page.click('#confirm-delete'), 'ok'],
        ['delete', () => page.click('#cancel-delete'), 'cancel'],
        ['delete', () => page.keyboard.press('Escape'), 'cancel'],
        ['delete', () => page.goBack(), 'cancel'],
        ['delete', () => page.evaluate(() => t.close('delete')), 'cancel'],
        ['about', () => page.click('#ok-about'), 'ok'],
        ['about', () => page.click('#x-about'), 'cancel'],
        ['about', () => page.mouse.click(5, 5), 'cancel'],
      ]
      for (const [id, close, expected] of cases) {
        await page.evaluate(id => { self.r = t.confirm(id) }, id)
        await waitOpen(id)
        await close()
        assert.equal(await page.evaluate(() => self.r), expected)
      }
      await waitPops(cases.length)
      assert.equal((await state()).navigations, 0)
      await backToStart()
    })

    it('dropdown opens next to the button, toggles and closes via item, Esc, outside and back', async () => {
      await page.click('#open-more')
      await waitOpen('more')
      const pos = await page.evaluate(() => {
        const b = document.getElementById('open-more').getBoundingClientRect()
        const m = document.getElementById('more').getBoundingClientRect()
        return Math.round(m.top - b.bottom)
      })
      assert.equal(pos, 4)
      assert.equal((await state()).lock, false)

      await page.goBack()
      await waitOpen('more', false)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-more' })

      let pops = 1
      for (const close of [
        () => page.click('#open-more'),
        () => page.keyboard.press('Escape'),
        () => page.click('#outside'),
        () => page.click('#item-copy'),
      ]) {
        await page.click('#open-more')
        await waitOpen('more')
        await close()
        await waitOpen('more', false)
        await waitPops(++pops)
      }
      assert.equal((await state()).focus, 'open-more')
      assert.equal((await state()).navigations, 0)
      await backToStart()
    })

    it('dropdown item opens the alert and history stays consistent', async () => {
      await page.click('#open-more')
      await page.click('#item-delete')
      await waitOpen('delete')
      await waitOpen('more', false)
      await waitPops(1)
      await page.click('#confirm-delete')
      await waitOpen('delete', false)
      await waitPops(2)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-more' })
      await backToStart()
    })

    it('confirm() called from inside the dropdown returns focus to the trigger', async () => {
      await page.click('#open-more')
      await page.click('#item-js')
      await waitOpen('delete')
      await page.click('#confirm-delete')
      await waitPops(2)
      assert.equal(await page.evaluate(() => self.r), 'ok')
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-more' })
      await backToStart()
    })

    it('closing via UI neither navigates nor piles up entries', async () => {
      for (let i = 0; i < 3; i++) {
        await page.click('#open-about')
        await page.click('#ok-about')
        await waitOpen('about', false)
        await waitPops(i + 1)
      }
      assert.equal((await state()).navigations, 0)
      await backToStart()
    })

    it('closeAll() closes everything without touching history', async () => {
      await page.click('#open-about')
      await page.click('#open-details')
      const r = await page.evaluate(() => { self.r = t.confirm('details'); return t.closeAll() })
      assert.equal(r, 2)
      assert.equal(await page.evaluate(() => self.r), 'cancel')
      await waitOpen('about', false)
      await waitOpen('details', false)
      await page.waitForTimeout(100)
      assert.equal(await page.evaluate(() => self.pops), 0)
      assert.equal((await state()).lock, false)
    })

    it('unknown id goes to log.error', async () => {
      await page.evaluate(() => t.open('nothing'))
      assert.deepEqual(await page.evaluate(() => t.errors), [['overlay not found', { 'pragma.overlay.id': 'nothing' }]])
    })
  })
}
