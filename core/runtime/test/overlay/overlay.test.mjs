import { describe, it, before, after, beforeEach, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { serve } from './server.mjs'

const pw = await import(process.env.PLAYWRIGHT ?? `${process.env.HOME}/.cache/pragma-tools/node_modules/playwright/index.mjs`)

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
      assert.deepEqual(await page.evaluate(() => [document.getElementById('delete').open, document.getElementById('delete').hasAttribute('data-leaving')]), [true, false])
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
      const wheeled = page.evaluate(() => new Promise(r => addEventListener('wheel', () => requestAnimationFrame(() => requestAnimationFrame(r)), { once: true })))
      await page.mouse.wheel(0, 800)
      await wheeled
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
      await page.evaluate(() => new Promise(r => {
        addEventListener('popstate', r, { once: true })
        history.pushState(history.state, '')
        history.back()
      }))
      assert.equal(await page.evaluate(() => self.pops), 1, 'only the probe entry was popped')
      assert.equal((await state()).lock, false)
    })

    it('closing plays the CSS exit animation, then closes and refocuses', async () => {
      await page.evaluate(() => document.documentElement.classList.add('motion'))
      await page.click('#open-about')
      await page.evaluate(() => { self.r = t.confirm('about') })
      const leaving = () => page.evaluate(() => {
        const d = document.getElementById('about')
        return [d.open, d.hasAttribute('data-leaving'), d.inert]
      })
      await page.click('#ok-about')
      assert.deepEqual(await leaving(), [true, true, true])
      assert.equal(await page.evaluate(() => self.r), 'ok')
      assert.equal((await state()).lock, false)
      await waitOpen('about', false)
      assert.deepEqual(await leaving(), [false, false, false])
      await waitPops(1)
      assert.deepEqual(await state(), { path: '/index.html', navigations: 0, lock: false, focus: 'open-about' })

      await page.click('#open-about')
      await page.keyboard.press('Escape')
      assert.deepEqual(await leaving(), [true, true, true])
      await waitOpen('about', false)
      await waitPops(2)
      assert.equal((await state()).focus, 'open-about')

      await page.click('#open-about')
      await page.click('#ok-about')
      await page.evaluate(() => t.open('about'))
      assert.deepEqual(await leaving(), [true, false, false])
      await page.waitForFunction(() => document.getElementById('about').getAnimations().length === 0)
      await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))))
      assert.equal(await page.evaluate(open, 'about'), true)
      await page.keyboard.press('Escape')
      await waitOpen('about', false)
      await waitPops(4)
      assert.equal((await state()).navigations, 0)
      await backToStart()
    })

    it('reduced motion closes at once', async () => {
      await page.emulateMedia({ reducedMotion: 'reduce' })
      await page.evaluate(() => document.documentElement.classList.add('motion'))
      await page.click('#open-about')
      await page.click('#ok-about')
      assert.equal(await page.evaluate(open, 'about'), false)
      await waitPops(1)
      assert.equal((await state()).focus, 'open-about')
      await backToStart()
    })

    it('focus on open: first focusable, or the dialog itself with autofocus', async () => {
      await page.click('#open-about')
      assert.equal((await state()).focus, 'x-about')
      await page.keyboard.press('Escape')
      await waitOpen('about', false)
      await page.evaluate(() => t.open('terms'))
      assert.equal((await state()).focus, 'terms')
      await page.click('#ok-terms')
      await waitOpen('terms', false)
      await waitPops(2)
    })

    for (const measured of [false, true]) it(`dropdown placement${measured ? ' (measured, without CSS anchor positioning)' : ''}: start or end edge, below or above, flips without room`, async () => {
      if (measured) {
        await page.addInitScript(() => { const supports = CSS.supports; CSS.supports = (...a) => a[0] != 'position-try-fallbacks' && supports.apply(CSS, a) })
        await page.reload()
        await page.waitForFunction(() => self.t)
      }
      const gap = async placement => {
        await page.evaluate(p => { document.getElementById('more').dataset.placement = p }, placement)
        await page.click('#open-more')
        await waitOpen('more')
        const r = await page.evaluate(() => {
          const b = document.getElementById('open-more').getBoundingClientRect()
          const m = document.getElementById('more').getBoundingClientRect()
          const n = v => Math.round(v) || 0
          return { below: n(m.top - b.bottom), above: n(b.top - m.bottom), left: n(m.left - b.left), right: n(m.right - b.right) }
        })
        await page.keyboard.press('Escape')
        await waitOpen('more', false)
        return r
      }
      await page.evaluate(() => Object.assign(document.querySelector('[data-c=dropdown]').style, { position: 'fixed', top: '200px', left: '300px' }))
      const at = async placement => {
        const r = await gap(placement)
        return [placement.startsWith('top') ? r.above : r.below, placement.endsWith('end') ? r.right : r.left]
      }
      for (const p of ['bottom-start', 'bottom-end', 'top-start', 'top-end']) assert.deepEqual(await at(p), [4, 0], p)

      await page.evaluate(() => Object.assign(document.querySelector('[data-c=dropdown]').style, { top: 'auto', left: 'auto', bottom: '8px', right: '8px' }))
      const flipped = await gap('bottom-start')
      assert.deepEqual([flipped.above, flipped.right], [4, 0])
      await page.evaluate(() => Object.assign(document.querySelector('[data-c=dropdown]').style, { top: '8px', left: '8px', bottom: 'auto', right: 'auto' }))
      const down = await gap('top-end')
      assert.deepEqual([down.below, down.left], [4, 0])
    })

    it('unknown id goes to log.error', async () => {
      await page.evaluate(() => t.open('nothing'))
      assert.deepEqual(await page.evaluate(() => t.errors), [['overlay not found', { 'pragma.overlay.id': 'nothing' }]])
    })
  })
}
