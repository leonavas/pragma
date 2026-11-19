import { describe, it, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { homedir, tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { gzipSync } from 'node:zlib'
import { fileURLToPath } from 'node:url'

const PRAGMA = process.env.PRAGMA ?? 'pragma'
const SERVE_PORT = Number(process.env.PACKAGES_PORT ?? 5283)
const DEV_PORT = Number(process.env.PACKAGES_DEV_PORT ?? 5284)
const TOOLS = process.env.PRAGMA_TOOLS ?? join(homedir(), '.cache/pragma-tools')
const BROWSERS = (process.env.BROWSERS ?? 'chromium,firefox').split(',')
const playwright = createRequire(process.env.PLAYWRIGHT_DIR ? join(process.env.PLAYWRIGHT_DIR, '/') : import.meta.url)('playwright')
const fixture = join(dirname(fileURLToPath(import.meta.url)), 'app')
const root = mkdtempSync(join(tmpdir(), 'pragma-packages-'))

const DATE_FNS_GZIP_LIMIT = 8 * 1024
const PAGE_GZIP_LIMIT = 25 * 1024
const PAGE_GZIP_LIMITS = { '/chart': 70 * 1024 }
const LATE_PACKAGE = { name: 'ms', version: '2.1.3' }
const LODASH_MARKER = '__lodash_hash_undefined__'

const PACKAGES = ['date-fns', '@date-fns/tz', 'es-toolkit', 'valibot', 'fuse.js', 'chart.js', 'dayjs', 'lodash']

const PAGES = {
  '/': ['date-fns'],
  '/timezone': ['@date-fns/tz', 'date-fns'],
  '/dayjs': ['dayjs'],
  '/lodash': ['lodash'],
  '/totals': ['lodash'],
  '/es-toolkit': ['es-toolkit'],
  '/valibot': ['valibot'],
  '/search': ['fuse.js'],
  '/chart': ['chart.js'],
}

const fileName = pkg => pkg.replace(/^@/, '').replaceAll('/', '__')
const escapeRegex = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const prodEntry = pkg => new RegExp(`^${escapeRegex(fileName(pkg))}-[A-Za-z0-9_-]{8}\\.js$`)
const devEntry = pkg => new RegExp(`^${escapeRegex(fileName(pkg))}\\.js$`)
const packageFile = pkg => new RegExp(`^${escapeRegex(fileName(pkg))}(\\.\\d+)?-[A-Za-z0-9_-]{8}\\.js$`)
const NAMES = {
  prod: { entry: prodEntry, any: { test: file => /^[A-Za-z0-9_.-]+-[A-Za-z0-9_-]{8}\.js$/.test(file) } },
  dev: { entry: devEntry, any: { test: file => /^_[A-Za-z0-9_.-]+\.js$/.test(file) || [...PACKAGES, 'ms'].some(pkg => devEntry(pkg).test(file)) } },
}

const sleep = ms => new Promise(r => setTimeout(r, ms))
const sha = content => createHash('sha256').update(content).digest('hex')
const kb = bytes => (bytes / 1024).toFixed(1) + ' kB'

function withStaticImports(dir, entry) {
  const seen = new Set()
  const pending = [entry]
  while (pending.length) {
    const file = pending.pop()
    if (seen.has(file)) continue
    seen.add(file)
    const code = readFileSync(join(dir, file), 'utf8')
    for (const [, specifier] of code.matchAll(/(?:^|[;\n])\s*(?:import|export)\s*(?:[\w$*{}\s,]+?\s*from\s*)?["']([^"']+)["']/g)) {
      if (specifier.startsWith('./') || specifier.startsWith('../')) pending.push(join(dirname(file), specifier))
      else if (specifier.startsWith('/assets/npm/')) pending.push(specifier.slice('/assets/npm/'.length))
    }
  }
  return [...seen]
}

function filesIn(dir) {
  return readdirSync(dir, { recursive: true }).filter(f => statSync(join(dir, f)).isFile()).sort()
}

function pragma(cwd, ...args) {
  const r = spawnSync(PRAGMA, args, { cwd, encoding: 'utf8' })
  if (r.error) throw r.error
  return { status: r.status, output: (r.stdout ?? '') + (r.stderr ?? '') }
}

function newApp(name) {
  const dir = join(root, name)
  const r = pragma(root, 'new', dir)
  assert.equal(r.status, 0, `pragma new failed:\n${r.output}`)
  return dir
}

function writeFiles(dir, files) {
  for (const [path, content] of Object.entries(files)) {
    const target = join(dir, path)
    mkdirSync(dirname(target), { recursive: true })
    writeFileSync(target, typeof content === 'string' ? content : JSON.stringify(content, null, 2))
  }
}

function npm(cwd, args, cache) {
  return spawnSync('npm', [...args, '--cache', cache, '--ignore-scripts', '--no-audit', '--no-fund', '--silent'], { cwd, encoding: 'utf8' })
}

let cached
function cachedPackages() {
  if (cached) return cached
  const lock = readFileSync(join(fixture, 'package-lock.json'))
  const key = createHash('sha256').update(lock).update(`${LATE_PACKAGE.name}@${LATE_PACKAGE.version}`).digest('hex').slice(0, 16)
  const dir = join(TOOLS, 'packages', key)
  cached = { nodeModules: join(dir, 'node_modules'), npmCache: join(dir, 'npm-cache') }
  if (existsSync(join(dir, 'ready'))) return cached
  const staging = `${dir}.${process.pid}`
  rmSync(staging, { recursive: true, force: true })
  mkdirSync(staging, { recursive: true })
  cpSync(join(fixture, 'package.json'), join(staging, 'package.json'))
  cpSync(join(fixture, 'package-lock.json'), join(staging, 'package-lock.json'))
  console.log(`installing npm packages in ${dir} (first run only; needs network)`)
  const stagingCache = join(staging, 'npm-cache')
  const ci = npm(staging, ['ci'], stagingCache)
  assert.equal(ci.status, 0, `npm ci failed:\n${ci.stdout}${ci.stderr}`)
  const add = npm(staging, ['cache', 'add', `${LATE_PACKAGE.name}@${LATE_PACKAGE.version}`], stagingCache)
  assert.equal(add.status, 0, `npm cache add failed:\n${add.stdout}${add.stderr}`)
  writeFileSync(join(staging, 'ready'), key)
  rmSync(dir, { recursive: true, force: true })
  renameSync(staging, dir)
  return cached
}

function prepareApp(name) {
  const dir = newApp(name)
  cpSync(fixture, dir, { recursive: true })
  cpSync(cachedPackages().nodeModules, join(dir, 'node_modules'), { recursive: true })
  return dir
}

function diagnostics(output) {
  return output.split('\n').filter(l => /\s(error|warning)\s\[[^\]]+\]/.test(l))
}

async function startServer(cmd, cwd, port) {
  const proc = spawn(PRAGMA, [cmd, '--port', String(port)], { cwd, stdio: ['ignore', 'pipe', 'pipe'] })
  let output = ''
  proc.stdout.on('data', d => { output += d })
  proc.stderr.on('data', d => { output += d })
  const url = `http://127.0.0.1:${port}`
  for (let attempt = 0; ; attempt++) {
    try {
      if ((await fetch(url + '/')).ok) break
    } catch {}
    if (proc.exitCode !== null || attempt > 150) {
      proc.kill('SIGINT')
      throw new Error(`pragma ${cmd} did not start:\n${output}`)
    }
    await sleep(100)
  }
  return {
    url,
    log: () => output,
    async stop() {
      if (proc.exitCode !== null) return
      proc.kill('SIGINT')
      await new Promise(r => proc.once('exit', r))
    },
  }
}

async function npmPreloads(url, route) {
  const html = await (await fetch(url + route)).text()
  return [...html.matchAll(/<link\b[^>]*rel="modulepreload"[^>]*>/g)]
    .map(m => m[0].match(/href="([^"]+)"/)?.[1] ?? '')
    .filter(href => href.startsWith('/assets/npm/'))
    .map(href => href.slice('/assets/npm/'.length))
    .sort()
}

async function pageScripts(url, route) {
  const html = await (await fetch(url + route)).text()
  const preloads = [...html.matchAll(/<link\b[^>]*rel="modulepreload"[^>]*href="([^"]+)"/g)].map(m => m[1])
  const scripts = [...html.matchAll(/<script\b[^>]*type="module"[^>]*src="([^"]+)"/g)].map(m => m[1])
  return [...new Set([...preloads, ...scripts])].filter(src => src.startsWith('/assets/'))
}

function expectedPreloads(npmDir, route, entryOf) {
  const files = readdirSync(npmDir)
  const entries = PAGES[route].map(pkg => {
    const entry = files.find(f => entryOf(pkg).test(f))
    assert.ok(entry, `${route}: entry of ${pkg} not found in ${npmDir}`)
    return entry
  })
  return [...new Set(entries.flatMap(entry => withStaticImports(npmDir, entry)))].sort()
}

const waitForText = (page, selector, text, timeout = 5000) =>
  page.waitForFunction(([s, t]) => document.querySelector(s)?.textContent === t, [selector, text], { timeout })
    .catch(async () => assert.fail(`${selector}: expected "${text}", got "${await page.locator(selector).textContent().catch(() => null)}"`))

const listTexts = (page, selector) => page.locator(selector).evaluateAll(items => items.map(item => item.textContent.trim()))

const waitForList = (page, selector, expected) =>
  page.waitForFunction(([s, e]) => JSON.stringify([...document.querySelectorAll(s)].map(i => i.textContent.trim())) === JSON.stringify(e), [selector, expected], { timeout: 5000 })
    .catch(async () => assert.fail(`${selector}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(await listTexts(page, selector))}`))

const chartState = page => page.evaluate(async () => {
  const link = [...document.querySelectorAll('link[rel="modulepreload"]')].find(l => /\/assets\/npm\/chart\.js[-.]/.test(l.getAttribute('href')))
  if (!link) return { error: 'no modulepreload of chart.js' }
  const { Chart } = await import(link.href)
  const canvas = document.querySelector('#sales-by-month')
  const chart = Chart.getChart(canvas)
  if (!chart) return { error: 'Chart.getChart(canvas) is undefined' }
  const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data
  let painted = 0
  let signature = 0
  for (let i = 0; i < pixels.length; i += 4) {
    if (pixels[i + 3] === 0) continue
    painted += 1
    signature = (Math.imul(signature, 31) + pixels[i] + pixels[i + 1] * 7 + pixels[i + 2] * 13 + i) >>> 0
  }
  return { type: chart.config.type, labels: [...chart.data.labels], data: [...chart.data.datasets[0].data], painted, signature }
})

const PAGE_CHECKS = [
  ['date-fns from the index', '/', async page => {
    await waitForText(page, '#formatted-date', '2024-01-15')
  }],
  ['date-fns pt-BR locale and @date-fns/tz conversion', '/timezone', async page => {
    await waitForText(page, '#tokyo-time', '21:00')
    await waitForText(page, '#sao-paulo-time', 'segunda-feira, 15 de janeiro de 2024, 09:00')
    await waitForText(page, '#converted-time', '2024-01-15 21:00')
  }],
  ['dayjs (CommonJS) with the relativeTime plugin', '/dayjs', async page => {
    await waitForText(page, '#shifted-date', '2024-03-15')
    await waitForText(page, '#relative-date', '5 days ago')
  }],
  ['lodash/groupBy (CommonJS)', '/lodash', async page => {
    await waitForText(page, '#fruits-by-initial', 'a: 2, g: 1, b: 1')
  }],
  ['lodash/groupBy and lodash/sumBy on a second page', '/totals', async page => {
    await waitForText(page, '#total-stock', '15')
    await waitForText(page, '#stock-groups', 'drink, food')
  }],
  ['es-toolkit: groupBy, and debounce saves once after rapid typing', '/es-toolkit', async page => {
    await waitForText(page, '#products-by-category', 'drinks: 2, bakery: 1')
    await waitForText(page, '#saves', '0')
    await page.getByLabel('Draft').pressSequentially('hello', { delay: 15 })
    await waitForText(page, '#saved-draft', 'hello')
    await sleep(400)
    assert.equal(await page.locator('#saves').textContent(), '1')
    await page.getByLabel('Draft').pressSequentially('!', { delay: 15 })
    await waitForText(page, '#saved-draft', 'hello!')
    await waitForText(page, '#saves', '2')
  }],
  ['valibot: invalid then valid email toggles the message', '/valibot', async page => {
    await waitForText(page, '#email-status', 'Valid email')
    await page.getByLabel('Email').fill('ana')
    await page.waitForFunction(() => document.querySelector('#email-status')?.textContent.startsWith('Invalid email'), null, { timeout: 5000 })
      .catch(async () => assert.fail(`expected "Invalid email…", got "${await page.locator('#email-status').textContent()}"`))
    await page.getByLabel('Email').fill('ana@example.org')
    await waitForText(page, '#email-status', 'Valid email')
  }],
  ['fuse.js: typing filters the results', '/search', async page => {
    await waitForList(page, '#results li', ['Roasted coffee', 'Green tea', 'Brown sugar', 'Black tea', 'Oat milk'])
    await page.getByLabel('Search products').pressSequentially('tea', { delay: 20 })
    await waitForList(page, '#results li', ['Green tea', 'Black tea'])
    await page.getByLabel('Search products').fill('gren')
    await waitForList(page, '#results li', ['Green tea'])
    await page.getByLabel('Search products').fill('zzz')
    await waitForList(page, '#results li', ['Nothing found'])
  }],
  ['chart.js: renders the dataset and re-renders after an action', '/chart', async page => {
    await page.waitForFunction(() => {
      const canvas = document.querySelector('#sales-by-month')
      const pixels = canvas?.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data ?? []
      for (let i = 3; i < pixels.length; i += 4) if (pixels[i] > 0) return true
      return false
    }, null, { timeout: 5000 }).catch(() => assert.fail('canvas is still blank'))
    const initial = await chartState(page)
    assert.equal(initial.error, undefined, initial.error)
    assert.deepEqual({ type: initial.type, labels: initial.labels, data: initial.data }, { type: 'bar', labels: ['Jan', 'Feb', 'Mar'], data: [12, 19, 7] })
    assert.ok(initial.painted > 1000, `only ${initial.painted} painted pixels`)
    await page.getByRole('button', { name: 'Add April' }).click()
    await waitForText(page, '#month-count', '4')
    const updated = await chartState(page)
    assert.deepEqual({ labels: updated.labels, data: updated.data }, { labels: ['Jan', 'Feb', 'Mar', 'Apr'], data: [12, 19, 7, 23] })
    assert.notEqual(updated.signature, initial.signature, 'canvas did not change after the update')
  }],
]

function browserSuites(mode, getUrl) {
  for (const name of BROWSERS) {
    describe(`${name} (${mode})`, () => {
      let browser

      before(async () => { browser = await playwright[name].launch() })
      after(() => browser?.close())

      async function open(route) {
        const url = getUrl()
        assert.ok(url, 'server did not start')
        const context = await browser.newContext({ baseURL: url, timezoneId: 'America/New_York', locale: 'en-US' })
        const page = await context.newPage()
        const errors = []
        const npmRequests = []
        page.on('pageerror', e => errors.push(`pageerror: ${e.message}`))
        page.on('console', m => m.type() === 'error' && errors.push(`console: ${m.text()}`))
        page.on('requestfailed', r => r.failure()?.errorText !== 'net::ERR_ABORTED' && !r.url().includes('/_pragma/reload') && errors.push(`requestfailed: ${r.url()}: ${r.failure()?.errorText}`))
        page.on('response', r => r.status() >= 400 && errors.push(`HTTP ${r.status()}: ${r.url()}`))
        page.on('request', r => {
          const path = new URL(r.url()).pathname
          if (path.includes('/npm/') || path.includes('node_modules')) npmRequests.push(path)
        })
        const response = await page.goto(route)
        assert.equal(response.status(), 200)
        return { context, page, errors, npmRequests }
      }

      for (const [title, route, run] of PAGE_CHECKS) {
        it(`${route}: ${title}`, async () => {
          const { context, page, errors, npmRequests } = await open(route)
          try {
            await run(page)
            await sleep(50)
          } finally {
            await context.close()
          }
          assert.deepEqual(errors, [])
          const names = NAMES[mode]
          for (const path of npmRequests) {
            assert.ok(path.startsWith('/assets/npm/') && names.any.test(path.slice('/assets/npm/'.length)), `${route}: unexpected npm request ${path}`)
          }
          for (const pkg of PAGES[route]) {
            assert.ok(npmRequests.some(p => names.entry(pkg).test(p.slice('/assets/npm/'.length))), `${route}: ${pkg} was not loaded from /assets/npm; requests: ${JSON.stringify(npmRequests)}`)
          }
        })
      }
    })
  }
}

after(() => rmSync(root, { recursive: true, force: true }))

describe('prod: pragma build and pragma serve', () => {
  let app, twin, server, url

  before(() => {
    app = prepareApp('app')
    twin = prepareApp('twin-build-in-another-directory')
  })

  after(() => server?.stop())

  const npmDir = () => join(app, 'dist/client/assets/npm')

  it('pragma check --strict has no errors or warnings', () => {
    const r = pragma(app, 'check', '--strict')
    assert.deepEqual(diagnostics(r.output), [], r.output)
    assert.equal(r.status, 0, r.output)
  })

  it('pragma build writes dist/', () => {
    const r = pragma(app, 'build')
    console.log(r.output.trim())
    assert.equal(r.status, 0, r.output)
  })

  it('two clean builds in different directories produce identical npm files', () => {
    const r = pragma(twin, 'build')
    assert.equal(r.status, 0, r.output)
    const digest = dir => Object.fromEntries(filesIn(dir).map(f => [f, sha(readFileSync(join(dir, f)))]))
    assert.deepEqual(digest(join(twin, 'dist/client/assets/npm')), digest(npmDir()))
  })

  it('two clean builds produce identical client assets', () => {
    const digest = dir => Object.fromEntries(filesIn(dir).map(f => [f, sha(readFileSync(join(dir, f)))]))
    assert.deepEqual(digest(join(twin, 'dist/client')), digest(join(app, 'dist/client')))
  })

  it('a rebuild with a warm cache keeps the same npm files', () => {
    const before = filesIn(npmDir())
    const r = pragma(app, 'build')
    assert.equal(r.status, 0, r.output)
    assert.deepEqual(filesIn(npmDir()), before)
  })

  it('one entry file per package in assets/npm', () => {
    const files = filesIn(npmDir()).filter(f => f.endsWith('.js'))
    const lines = files.map(f => {
      const content = readFileSync(join(npmDir(), f))
      return `  ${f.padEnd(40)} ${kb(content.length).padStart(9)} ${kb(gzipSync(content).length).padStart(9)} gzip`
    })
    console.log(`assets/npm (${files.length} files):\n${lines.join('\n')}`)
    for (const pkg of PACKAGES) {
      const entries = files.filter(f => packageFile(pkg).test(f))
      assert.equal(entries.length, 1, `${pkg}: expected 1 entry ${fileName(pkg)}-<hash8>.js, got ${JSON.stringify(entries)}`)
    }
    const strays = files.filter(f => !f.startsWith('_') && !PACKAGES.some(pkg => packageFile(pkg).test(f)))
    assert.deepEqual(strays, [], 'files in assets/npm that belong to no package and are not shared chunks')
  })

  it(`tree-shaking: date-fns (format + ptBR locale) stays under ${kb(DATE_FNS_GZIP_LIMIT)} gzip`, () => {
    const entry = readdirSync(npmDir()).find(f => prodEntry('date-fns').test(f))
    assert.ok(entry, 'date-fns entry not found')
    const files = withStaticImports(npmDir(), entry)
    const size = gzipSync(Buffer.concat(files.map(f => readFileSync(join(npmDir(), f))))).length
    console.log(`date-fns: ${kb(size)} gzip (${files.join(', ')})`)
    assert.ok(size < DATE_FNS_GZIP_LIMIT, `date-fns is ${kb(size)} gzip`)
  })

  it('lodash (CommonJS subpaths) is bundled once and shared by both pages', () => {
    const lodash = readdirSync(npmDir()).filter(f => packageFile('lodash').test(f))
    assert.equal(lodash.length, 1, JSON.stringify(lodash))
    const content = readFileSync(join(npmDir(), lodash[0]))
    const whole = readFileSync(join(cachedPackages().nodeModules, 'lodash/lodash.min.js'))
    console.log(`lodash/groupBy + lodash/sumBy: ${kb(content.length)} (${kb(gzipSync(content).length)} gzip); whole lodash.min.js: ${kb(whole.length)} (${kb(gzipSync(whole).length)} gzip)`)
    const assets = join(app, 'dist/client/assets')
    const withMarker = filesIn(assets).filter(f => f.endsWith('.js') && readFileSync(join(assets, f), 'utf8').includes(LODASH_MARKER))
    assert.deepEqual(withMarker, [`npm/${lodash[0]}`], 'lodash internals must live in exactly one file')
  })

  it('pragma serve starts', async () => {
    server = await startServer('serve', app, SERVE_PORT)
    url = server.url
  })

  it('each page modulepreloads exactly its packages and their shared chunks', async () => {
    assert.ok(url, 'server did not start')
    for (const route of Object.keys(PAGES)) {
      const preloads = await npmPreloads(url, route)
      assert.deepEqual(preloads, expectedPreloads(npmDir(), route, prodEntry), `${route}: npm modulepreloads`)
    }
    assert.deepEqual(await npmPreloads(url, '/lodash'), await npmPreloads(url, '/totals'))
  })

  it('JS gzip per page stays under its cap', async () => {
    assert.ok(url, 'server did not start')
    const client = join(app, 'dist/client')
    const rows = []
    for (const route of Object.keys(PAGES)) {
      const scripts = await pageScripts(url, route)
      const total = scripts.reduce((sum, src) => sum + gzipSync(readFileSync(join(client, src))).length, 0)
      const npmTotal = scripts.filter(s => s.startsWith('/assets/npm/')).reduce((sum, src) => sum + gzipSync(readFileSync(join(client, src))).length, 0)
      rows.push([route, total, npmTotal, PAGE_GZIP_LIMITS[route] ?? PAGE_GZIP_LIMIT])
    }
    console.log(`JS gzip per page (sum of files):\n${rows.map(([r, t, n, cap]) => `  ${r.padEnd(12)} ${kb(t).padStart(9)} total ${kb(n).padStart(9)} npm   cap ${kb(cap)}`).join('\n')}`)
    for (const [route, total, , cap] of rows) assert.ok(total < cap, `${route}: ${kb(total)} gzip >= cap ${kb(cap)}`)
  })

  browserSuites('prod', () => url)
})

describe('dev: pragma dev', () => {
  let app, server, url

  before(async () => {
    app = prepareApp('dev-app')
    server = await startServer('dev', app, DEV_PORT)
    url = server.url
  })

  after(async () => {
    const log = server?.log() ?? ''
    await server?.stop()
    assert.doesNotMatch(log, /panic|compiler crashed/i, log)
  })

  const npmDir = () => join(app, '.pragma/dev/client/assets/npm')

  it('npm assets use stable names under /assets/npm and each page preloads exactly its packages', async () => {
    for (const route of Object.keys(PAGES)) {
      const preloads = await npmPreloads(url, route)
      for (const file of preloads) assert.ok(NAMES.dev.any.test(file), `${route}: dev npm file ${file} should be <package>.js or _<chunk>.js, without a hash`)
      assert.deepEqual(preloads, expectedPreloads(npmDir(), route, devEntry), `${route}: npm modulepreloads`)
    }
    for (const pkg of PACKAGES) {
      const response = await fetch(`${url}/assets/npm/${fileName(pkg)}.js`)
      assert.equal(response.status, 200, `${pkg}: /assets/npm/${fileName(pkg)}.js`)
      assert.match(response.headers.get('content-type') ?? '', /javascript/)
    }
  })

  browserSuites('dev', () => url)

  describe('chromium (pragma dev live reload)', () => {
    let browser, context, page
    const errors = []
    const indexFile = () => join(app, 'routes/index.js')
    const original = readFileSync(join(fixture, 'routes/index.js'), 'utf8')

    const withAddDays = original
      .replace("import { format } from 'date-fns'", "import { addDays, format } from 'date-fns'")
      .replace('format(new Date(2024, 0, 15),', 'format(addDays(new Date(2024, 0, 15), 7),')
    const withMs = withAddDays
      .replace("from 'date-fns'\n", `from 'date-fns'\nimport ms from '${LATE_PACKAGE.name}'\n`)
      .replace("'yyyy-MM-dd')\n", "'yyyy-MM-dd') + ' / ' + ms('2 days')\n")
    const withUnlisted = withMs
      .replace(`import ms from '${LATE_PACKAGE.name}'\n`, `import ms from '${LATE_PACKAGE.name}'\nimport { rgbString } from '@kurkle/color'\n`)
      .replace("ms('2 days')\n", "ms('2 days') + rgbString({ r: 0, g: 0, b: 0, a: 255 })\n")

    const overlayText = () => page.evaluate(() => [...document.documentElement.children].find(e => e.textContent.includes('pragma: build error'))?.textContent ?? null)

    async function waitForOverlay(...snippets) {
      await page.waitForFunction(s => {
        const text = [...document.documentElement.children].find(e => e.textContent.includes('pragma: build error'))?.textContent ?? ''
        return s.every(t => text.includes(t))
      }, snippets, { timeout: 10000 }).catch(async () => assert.fail(`overlay with ${JSON.stringify(snippets)} did not appear; overlay: ${await overlayText()}\nserver log:\n${server.log()}`))
    }

    async function markPage() {
      await page.evaluate(() => { self.__beforeReload = true })
    }

    async function waitForReloadWith(text) {
      await page.waitForFunction(t => self.__beforeReload === undefined && document.querySelector('#formatted-date')?.textContent === t, text, { timeout: 10000 })
        .catch(async () => assert.fail(`page did not reload with "${text}"; got "${await page.locator('#formatted-date').textContent().catch(() => null)}"\nserver log:\n${server.log()}`))
      assert.equal(await overlayText(), null, 'overlay still visible after the fix')
    }

    before(async () => {
      assert.ok(withAddDays !== original && withMs !== withAddDays && withUnlisted !== withMs, 'fixture routes/index.js changed; update the live reload edits')
      browser = await playwright.chromium.launch()
      context = await browser.newContext({ baseURL: url })
      page = await context.newPage()
      page.on('pageerror', e => errors.push(`pageerror: ${e.message}`))
      page.on('console', m => m.type() === 'error' && errors.push(`console: ${m.text()}`))
      page.on('response', r => r.status() >= 400 && errors.push(`HTTP ${r.status()}: ${r.url()}`))
      await page.goto('/')
      await waitForText(page, '#formatted-date', '2024-01-15')
    })

    after(async () => {
      await browser?.close()
    })

    it('a new named import from an installed package rebuilds and reloads', async () => {
      const preloadsBefore = await npmPreloads(url, '/')
      await markPage()
      writeFileSync(indexFile(), withAddDays)
      await waitForReloadWith('2024-01-22')
      assert.deepEqual(await npmPreloads(url, '/'), preloadsBefore, 'npm file names changed after a rebuild')
      const dateFns = await (await fetch(`${url}/assets/npm/date-fns.js`)).text()
      assert.match(dateFns, /addDays/)
      assert.deepEqual(errors, [])
    })

    it('importing a package before installing it shows js/import; npm install fixes it through package.json', async () => {
      await markPage()
      writeFileSync(indexFile(), withMs)
      await waitForOverlay('[js/import]', `"${LATE_PACKAGE.name}"`, 'routes/index.js')
      const { npmCache } = cachedPackages()
      const r = npm(app, ['install', '--offline', '--save-exact', `${LATE_PACKAGE.name}@${LATE_PACKAGE.version}`], npmCache)
      assert.equal(r.status, 0, `npm install --offline failed:\n${r.stdout}${r.stderr}`)
      assert.equal(JSON.parse(readFileSync(join(app, 'package.json'), 'utf8')).dependencies[LATE_PACKAGE.name], LATE_PACKAGE.version)
      await waitForReloadWith('2024-01-22 / 172800000')
      assert.ok((await npmPreloads(url, '/')).includes(`${fileName(LATE_PACKAGE.name)}.js`), 'no modulepreload of the new package')
      assert.deepEqual(errors, [])
    })

    it('importing an unlisted package shows the overlay; reverting recovers', async () => {
      await markPage()
      writeFileSync(indexFile(), withUnlisted)
      await waitForOverlay('[js/import]', '@kurkle/color', 'routes/index.js')
      assert.equal(await page.evaluate(() => self.__beforeReload), true, 'the page reloaded on a failed build')
      writeFileSync(indexFile(), withMs)
      await waitForReloadWith('2024-01-22 / 172800000')
      assert.deepEqual(errors, [])
    })
  })
})

describe('rejected packages (pragma check)', () => {
  let base

  before(() => {
    base = newApp('base-rejected')
  })

  function check(name, files) {
    const dir = join(root, name)
    cpSync(base, dir, { recursive: true })
    writeFiles(dir, files)
    const r = pragma(dir, 'check')
    return { status: r.status, errors: diagnostics(r.output), output: r.output }
  }

  function expectError(outcome, rule, ...snippets) {
    assert.notEqual(outcome.status, 0, `check should fail:\n${outcome.output}`)
    const found = outcome.errors.some(l => l.includes(` error [${rule}]`) && snippets.some(t => l.includes(t)))
    assert.ok(found, `expected error [${rule}] mentioning ${snippets.join(' or ')}:\n${outcome.output}`)
  }

  const page = (binding, importLine) => ({
    'routes/rejected.html': `<script>\n  ${importLine}\n\n  const label = String(${binding})\n</script>\n\n<title>Rejected</title>\n\n<h1>Rejected</h1>\n<p>{$label}</p>\n`,
  })

  const fakePackage = (name, manifest, files) => Object.fromEntries([
    [`node_modules/${name}/package.json`, { name, version: '1.0.0', ...manifest }],
    ...Object.entries(files).map(([file, content]) => [`node_modules/${name}/${file}`, content]),
  ])

  it('UI framework (react) is js/import', () => {
    expectError(check('react', {
      'package.json': { name: 'rejected', private: true, dependencies: { react: '19.0.0' } },
      ...fakePackage('react', { type: 'module', main: 'index.js' }, { 'index.js': 'export const createElement = () => null\n' }),
      ...page('createElement', "import { createElement } from 'react'"),
    }), 'js/import', 'react')
  })

  it('package missing from dependencies is js/import', () => {
    expectError(check('not-listed', {
      'package.json': { name: 'rejected', private: true, dependencies: {} },
      ...fakePackage('currency-formatter', { type: 'module', main: 'index.js' }, { 'index.js': "export const formatCurrency = amount => '$' + amount\n" }),
      ...page('formatCurrency', "import { formatCurrency } from 'currency-formatter'"),
    }), 'js/import', 'currency-formatter')
  })

  it('package with a .ts entry is app/package', () => {
    expectError(check('typescript', {
      'package.json': { name: 'rejected', private: true, dependencies: { 'ts-adder': '1.0.0' } },
      ...fakePackage('ts-adder', { type: 'module', main: 'index.ts' }, { 'index.ts': 'export const add = (a: number, b: number): number => a + b\n' }),
      ...page('add', "import { add } from 'ts-adder'"),
    }), 'app/package', 'ts-adder', 'index.ts')
  })

  it('package importing node:fs is app/package', () => {
    expectError(check('node-fs', {
      'package.json': { name: 'rejected', private: true, dependencies: { 'file-reader': '1.0.0' } },
      ...fakePackage('file-reader', { type: 'module', main: 'index.js' }, {
        'index.js': "import { readFileSync } from 'node:fs'\n\nexport const readTextFile = path => readFileSync(path, 'utf8')\n",
      }),
      ...page('readTextFile', "import { readTextFile } from 'file-reader'"),
    }), 'app/package', 'file-reader', 'node:fs')
  })
})
