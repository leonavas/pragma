import { describe, it, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { browsers, launch, start } from './app.mjs'

const PORT = Number(process.env.DEV_PORT ?? 5282)
let app

before(async () => { app = await start('dev', PORT) })
after(() => app?.stop())

const edit = (file, from, to) => {
  const path = join(app.dir, file)
  const source = readFileSync(path, 'utf8')
  assert.ok(source.includes(from), `${file} does not contain ${from}`)
  writeFileSync(path, source.replace(from, to))
}

for (const name of browsers) {
  describe(`${name} (pragma dev)`, () => {
    let browser, page

    before(async () => {
      browser = await launch(name)
      page = await browser.newPage({ baseURL: app.url })
    })
    after(() => browser?.close())

    it('deep link with SSR, loader and directives in dev mode', async () => {
      await page.goto('/shop/coffee')
      assert.equal(await page.locator('main h1').textContent(), 'Ground coffee 500 g')
      assert.equal(await page.evaluate(() => self.__pragmaDev), true)
      await page.goto('/shop')
      await page.locator('main a[href="/shop/coffee"]').waitFor()
    })

    it('live reload: CSS swaps without reloading; HTML reloads', async () => {
      await page.goto('/')
      await page.evaluate(() => { self.__marker = 1 })
      const radius = () => page.evaluate(() => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius)
      const originalRadius = await radius()
      edit('lib/theme.css', 'border-radius: var(--radius-2xl);', 'border-radius: 3px;')
      await page.waitForFunction(() => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius === '3px')
      assert.equal(await page.evaluate(() => self.__marker), 1)
      edit('lib/theme.css', 'border-radius: 3px;', 'border-radius: var(--radius-2xl);')
      await page.waitForFunction(original => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius === original, originalRadius)

      edit('routes/index.html', '>Dashboard</h1>', '>Live dashboard</h1>')
      await page.waitForFunction(() => document.querySelector('main h1').textContent === 'Live dashboard')
      assert.equal(await page.evaluate(() => self.__marker), undefined)
      edit('routes/index.html', '>Live dashboard</h1>', '>Dashboard</h1>')
      await page.waitForFunction(() => document.querySelector('main h1').textContent === 'Dashboard')
    })
  })
}
