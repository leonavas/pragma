import { describe, it, before, after, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { browsers, launch, start, watchErrors } from './app.mjs'
import { appSuite } from './suite.mjs'

const PORT = Number(process.env.DEV_PORT ?? 5282)
let app

before(async () => { app = await start('dev', PORT) })
after(() => app?.stop())

appSuite('dev', () => app)

const originals = new Map()
const edit = (file, from, to) => {
  const path = join(app.dir, file)
  const source = readFileSync(path, 'utf8')
  assert.ok(source.includes(from), `${file} does not contain ${from}`)
  if (!originals.has(path)) originals.set(path, source)
  writeFileSync(path, source.replace(from, to))
}
const pendingRestores = () => [...originals].filter(([path, source]) => readFileSync(path, 'utf8') !== source)

for (const name of browsers) {
  describe(`${name} (pragma dev live reload)`, () => {
    let browser, page, errors

    before(async () => {
      browser = await launch(name)
      page = await browser.newPage({ baseURL: app.url })
      errors = []
      watchErrors(page, errors)
    })
    after(() => browser?.close())
    afterEach(async () => {
      const pending = pendingRestores()
      originals.clear()
      if (!pending.length) return
      await page.evaluate(() => { self.__marker = 1 })
      for (const [path, source] of pending) writeFileSync(path, source)
      await page.waitForFunction(() => self.__marker === undefined && document.querySelector('main h1'))
    })

    const marker = () => page.evaluate(() => self.__marker)
    const mark = () => page.evaluate(() => { self.__marker = 1 })
    const h1Is = text => page.waitForFunction(expected => document.querySelector('main h1')?.textContent === expected, text)

    it('dev mode flag and dev client are present', async () => {
      await page.goto('/shop/coffee')
      assert.equal(await page.locator('main h1').textContent(), 'Ground coffee 500 g')
      assert.equal(await page.evaluate(() => self.__pragmaDev), true)
      assert.deepEqual(errors, [])
    })

    it('CSS swaps without reloading', async () => {
      await page.goto('/')
      await mark()
      const radius = () => page.evaluate(() => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius)
      const originalRadius = await radius()
      edit('lib/theme.css', 'border-radius: var(--radius-2xl);', 'border-radius: 3px;')
      await page.waitForFunction(() => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius === '3px')
      assert.equal(await marker(), 1)
      edit('lib/theme.css', 'border-radius: 3px;', 'border-radius: var(--radius-2xl);')
      await page.waitForFunction(original => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius === original, originalRadius)
      assert.equal(await marker(), 1)
    })

    it('page HTML edit reloads', async () => {
      await page.goto('/')
      await mark()
      edit('routes/index.html', '>Dashboard</h1>', '>Live dashboard</h1>')
      await h1Is('Live dashboard')
      assert.equal(await marker(), undefined)
      edit('routes/index.html', '>Live dashboard</h1>', '>Dashboard</h1>')
      await h1Is('Dashboard')
    })

    it('page JS edit reloads with the new behavior', async () => {
      await page.goto('/tasks')
      await mark()
      edit('routes/tasks/index.js', "'Use at least 3 characters.'", "'Too short.'")
      await page.waitForFunction(() => self.__marker === undefined && /\d of \d/.test(document.querySelector('[data-c="indicator"] p')?.textContent))
      await page.getByRole('textbox', { name: 'New task' }).fill('ab')
      await page.getByRole('textbox', { name: 'New task' }).press('Enter')
      await page.locator('#task-error', { hasText: 'Too short.' }).waitFor()
      await mark()
      edit('routes/tasks/index.js', "'Too short.'", "'Use at least 3 characters.'")
      await page.waitForFunction(() => self.__marker === undefined && /\d of \d/.test(document.querySelector('[data-c="indicator"] p')?.textContent))
    })

    it('shared component edit reloads every page that uses it', async () => {
      await page.goto('/')
      await mark()
      edit('lib/components/indicator/indicator.html', '<h3 class="text-sm text-muted"', '<h3 class="text-xs text-muted"')
      await page.waitForFunction(() => self.__marker === undefined && document.querySelectorAll('[data-c="indicator"] h3.text-xs').length === 3)
      await page.goto('/tasks')
      await page.locator('[data-c="indicator"] h3.text-xs').waitFor()
    })

    it('build error shows the overlay; fixing it recovers', async () => {
      await page.goto('/')
      await mark()
      edit('routes/index.html', '<h2 id="summary" class="text-lg font-semibold">Summary</h2>', '<h2 id="summary" class="text-lg font-semibold">Summary')
      await page.getByText('pragma: build error').waitFor()
      assert.match(await page.locator('pre').last().textContent(), /routes\/index\.html/)
      edit('routes/index.html', '<h2 id="summary" class="text-lg font-semibold">Summary', '<h2 id="summary" class="text-lg font-semibold">Summary</h2>')
      await page.waitForFunction(() => self.__marker === undefined && document.querySelector('main h1')?.textContent === 'Dashboard')
      assert.equal(await page.getByText('pragma: build error').count(), 0)
    })
  })
}
