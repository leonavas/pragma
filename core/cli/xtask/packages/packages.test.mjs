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
const PORT = Number(process.env.PACKAGES_PORT ?? 5283)
const TOOLS = process.env.PRAGMA_TOOLS ?? join(homedir(), '.cache/pragma-tools')
const playwright = createRequire(process.env.PLAYWRIGHT_DIR ? join(process.env.PLAYWRIGHT_DIR, '/') : import.meta.url)('playwright')
const fixture = join(dirname(fileURLToPath(import.meta.url)), 'app')
const root = mkdtempSync(join(tmpdir(), 'pragma-packages-'))
const DATE_FNS_GZIP_LIMIT = 8 * 1024

const PACKAGES = ['date-fns', '@date-fns/tz', 'es-toolkit', 'valibot', 'fuse.js', 'chart.js', 'dayjs', 'lodash']

const PAGES = {
  '/': ['date-fns'],
  '/timezone': ['@date-fns/tz', 'date-fns'],
  '/dayjs': ['dayjs'],
  '/lodash': ['lodash'],
  '/es-toolkit': ['es-toolkit'],
  '/valibot': ['valibot'],
  '/search': ['fuse.js'],
  '/chart': ['chart.js'],
}

const fileName = pkg => pkg.replace(/^@/, '').replaceAll('/', '__')
const escapeRegex = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const entryOf = pkg => new RegExp(`^${escapeRegex(fileName(pkg))}-[A-Za-z0-9_-]{8}\\.js$`)
const packageFile = pkg => new RegExp(`^${escapeRegex(fileName(pkg))}(\\.\\d+)?-[A-Za-z0-9_-]{8}\\.js$`)

function withStaticImports(dir, entry) {
  const seen = new Set()
  const pending = [entry]
  while (pending.length) {
    const file = pending.pop()
    if (seen.has(file)) continue
    seen.add(file)
    const code = readFileSync(join(dir, file), 'utf8')
    for (const [, specifier] of code.matchAll(/(?:\bfrom\s*|\bimport\s*)["']([^"']+)["']/g)) {
      if (specifier.startsWith('./') || specifier.startsWith('../')) pending.push(join(dirname(file), specifier))
      else if (specifier.startsWith('/assets/npm/')) pending.push(specifier.slice('/assets/npm/'.length))
    }
  }
  return [...seen]
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

function cachedNodeModules() {
  const lock = readFileSync(join(fixture, 'package-lock.json'))
  const key = createHash('sha256').update(lock).digest('hex').slice(0, 16)
  const dir = join(TOOLS, 'packages', key)
  if (existsSync(join(dir, 'node_modules'))) return join(dir, 'node_modules')
  const staging = `${dir}.${process.pid}`
  rmSync(staging, { recursive: true, force: true })
  mkdirSync(staging, { recursive: true })
  cpSync(join(fixture, 'package.json'), join(staging, 'package.json'))
  cpSync(join(fixture, 'package-lock.json'), join(staging, 'package-lock.json'))
  console.log(`installing npm packages in ${dir} (first run only; needs network)`)
  const r = spawnSync('npm', ['ci', '--ignore-scripts', '--no-audit', '--no-fund', '--silent'], { cwd: staging, stdio: 'inherit' })
  assert.equal(r.status, 0, 'npm ci failed')
  rmSync(dir, { recursive: true, force: true })
  renameSync(staging, dir)
  return join(dir, 'node_modules')
}

function diagnostics(output) {
  return output.split('\n').filter(l => /\s(error|warning)\s\[[^\]]+\]/.test(l))
}

function kb(bytes) {
  return (bytes / 1024).toFixed(1) + ' kB'
}

after(() => rmSync(root, { recursive: true, force: true }))

describe('app with real npm packages', () => {
  let app, server, serverOutput = '', url

  before(() => {
    app = newApp('app')
    cpSync(fixture, app, { recursive: true })
    cpSync(cachedNodeModules(), join(app, 'node_modules'), { recursive: true })
  })

  after(async () => {
    if (!server) return
    server.kill('SIGINT')
    await new Promise(r => server.once('exit', r))
  })

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

  it('one entry file per package in assets/npm', () => {
    const dir = join(app, 'dist/client/assets/npm')
    assert.ok(existsSync(dir), 'dist/client/assets/npm does not exist')
    const files = readdirSync(dir, { recursive: true }).filter(f => f.endsWith('.js') && statSync(join(dir, f)).isFile())
    const lines = files.map(f => {
      const content = readFileSync(join(dir, f))
      return `  ${f.padEnd(40)} ${kb(statSync(join(dir, f)).size).padStart(9)} ${kb(gzipSync(content).length).padStart(9)} gzip`
    })
    console.log(`assets/npm (${files.length} files):\n${lines.slice(0, 40).join('\n')}${lines.length > 40 ? '\n  ...' : ''}`)
    for (const pkg of PACKAGES) {
      const entries = files.filter(f => packageFile(pkg).test(f))
      assert.equal(entries.length, 1, `${pkg}: expected 1 entry ${fileName(pkg)}-<hash8>.js, got ${JSON.stringify(entries)}`)
    }
  })

  it(`tree-shaking: date-fns with only format stays under ${kb(DATE_FNS_GZIP_LIMIT)} gzip`, () => {
    const dir = join(app, 'dist/client/assets/npm')
    const entry = readdirSync(dir).find(f => entryOf('date-fns').test(f))
    assert.ok(entry, 'date-fns entry not found')
    const files = withStaticImports(dir, entry)
    const size = gzipSync(Buffer.concat(files.map(f => readFileSync(join(dir, f))))).length
    console.log(`date-fns: ${kb(size)} gzip (${files.join(', ')})`)
    assert.ok(size < DATE_FNS_GZIP_LIMIT, `date-fns is ${kb(size)} gzip`)
  })

  it('pragma serve starts', async () => {
    server = spawn(PRAGMA, ['serve', '--port', String(PORT)], { cwd: app, stdio: ['ignore', 'pipe', 'pipe'] })
    server.stdout.on('data', d => { serverOutput += d })
    server.stderr.on('data', d => { serverOutput += d })
    url = `http://127.0.0.1:${PORT}`
    for (let attempt = 0; ; attempt++) {
      try {
        if ((await fetch(url + '/')).ok) break
      } catch {}
      assert.ok(attempt < 100, `serve did not start:\n${serverOutput}`)
      await new Promise(r => setTimeout(r, 100))
    }
  })

  it('each page modulepreloads the entries of its packages', async () => {
    assert.ok(url, 'server did not start')
    for (const [route, packages] of Object.entries(PAGES)) {
      const html = await (await fetch(url + route)).text()
      const preloads = [...html.matchAll(/<link\b[^>]*rel="modulepreload"[^>]*>/g)]
        .map(m => m[0].match(/href="([^"]+)"/)?.[1] ?? '')
      const npmPreloads = preloads.filter(href => href.startsWith('/assets/npm/'))
      for (const pkg of packages) {
        const found = npmPreloads.some(href => entryOf(pkg).test(href.slice('/assets/npm/'.length)))
        assert.ok(found, `${route}: no modulepreload of ${fileName(pkg)}-<hash8>.js; ${npmPreloads.length} assets/npm preloads, e.g. ${JSON.stringify(npmPreloads.slice(0, 3))}`)
      }
    }
  })

  describe('Chromium', () => {
    let browser, context

    before(async () => {
      browser = await playwright.chromium.launch()
    })
    after(() => browser?.close())

    async function open(route) {
      assert.ok(url, 'server did not start')
      context = await browser.newContext({ baseURL: url })
      const page = await context.newPage()
      const errors = []
      page.on('pageerror', e => errors.push(e.message))
      page.on('console', m => m.type() === 'error' && errors.push(m.text()))
      page.on('requestfailed', r => r.failure()?.errorText !== 'net::ERR_ABORTED' && errors.push(`${r.url()}: ${r.failure()?.errorText}`))
      const response = await page.goto(route)
      assert.equal(response.status(), 200)
      return { page, errors }
    }

    async function close(errors) {
      await context.close()
      assert.deepEqual(errors, [])
    }

    const waitForText = (page, selector, text) =>
      page.waitForFunction(([s, t]) => document.querySelector(s)?.textContent === t, [selector, text], { timeout: 5000 })
        .catch(async () => assert.fail(`${selector}: expected "${text}", got "${await page.locator(selector).textContent()}"`))

    it('date-fns from the index', async () => {
      const { page, errors } = await open('/')
      await waitForText(page, '#formatted-date', '2024-01-15')
      await close(errors)
    })

    it('@date-fns/tz with date-fns/format', async () => {
      const { page, errors } = await open('/timezone')
      await waitForText(page, '#tokyo-time', '21:00')
      await close(errors)
    })

    it('dayjs (CommonJS)', async () => {
      const { page, errors } = await open('/dayjs')
      await waitForText(page, '#shifted-date', '2024-03-15')
      await close(errors)
    })

    it('lodash/groupBy (CommonJS)', async () => {
      const { page, errors } = await open('/lodash')
      await waitForText(page, '#fruits-by-initial', 'a: 2, g: 1, b: 1')
      await close(errors)
    })

    it('es-toolkit: groupBy and debounce', async () => {
      const { page, errors } = await open('/es-toolkit')
      await waitForText(page, '#products-by-category', 'drinks: 2, bakery: 1')
      const button = page.getByRole('button', { name: 'Click' })
      await button.click()
      await button.click()
      await button.click()
      await waitForText(page, '#clicks', '3')
      await waitForText(page, '#saves', '1')
      await new Promise(r => setTimeout(r, 300))
      assert.equal(await page.locator('#saves').textContent(), '1')
      await close(errors)
    })

    it('valibot validates the typed email', async () => {
      const { page, errors } = await open('/valibot')
      await waitForText(page, '#email-status', 'Valid email')
      await page.getByLabel('Email').fill('ana')
      await page.waitForFunction(() => document.querySelector('#email-status')?.textContent.startsWith('Invalid email'), null, { timeout: 5000 })
      await close(errors)
    })

    it('fuse.js fuzzy search', async () => {
      const { page, errors } = await open('/search')
      await waitForText(page, '#found-product', 'Roasted coffee')
      await page.getByLabel('Search products').fill('gren')
      await waitForText(page, '#found-product', 'Green tea')
      await close(errors)
    })

    it('chart.js draws on the canvas', async () => {
      const { page, errors } = await open('/chart')
      await page.waitForFunction(() => {
        const canvas = document.querySelector('#sales-by-month')
        const pixels = canvas?.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data ?? []
        for (let i = 3; i < pixels.length; i += 4) if (pixels[i] > 0) return true
        return false
      }, null, { timeout: 5000 }).catch(() => assert.fail('canvas is still blank'))
      await close(errors)
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
    'routes/rejected.html': '<title>Rejected</title>\n\n<h1>Rejected</h1>\n<p :text="$label"></p>\n',
    'routes/rejected.js': `${importLine}\n\nexport const state = () => ({ label: String(${binding}) })\n`,
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
