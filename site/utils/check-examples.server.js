import { cp, mkdir, writeFile, readFile, readdir, rm, access } from 'node:fs/promises'
import { execFileSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'

const sections = [
  ['syntax', () => import('../routes/examples/syntax/index.server.js')],
  ['components', () => import('../routes/examples/components/index.server.js')],
  ['state', () => import('../routes/examples/state/index.server.js')],
  ['stores', () => import('../routes/examples/stores/index.server.js')],
  ['styling', () => import('../routes/examples/styling/index.server.js')],
  ['layout', () => import('../routes/examples/layout/index.server.js')],
  ['routing', () => import('../routes/examples/routing/index.server.js')],
  ['data', () => import('../routes/examples/data/index.server.js')],
  ['forms', () => import('../routes/examples/forms/index.server.js')],
  ['overlays', () => import('../routes/examples/overlays/index.server.js')],
  ['packages', () => import('../routes/examples/packages/index.server.js')],
  ['performance', () => import('../routes/examples/performance/index.server.js')],
  ['logs', () => import('../routes/examples/logs/index.server.js')],
  ['seo', () => import('../routes/examples/seo/index.server.js')],
  ['webmcp', () => import('../routes/examples/webmcp/index.server.js')],
]
const skipped = new Set(['syntax/rejected', 'state/mistakes', 'seo/rejected', 'seo/missing',
  'webmcp/rejected'])
const packages = ['valibot', 'dayjs', 'es-toolkit']
const workRoot = join(tmpdir(), 'pragma-check-examples')
const baseApp = join(workRoot, 'base')
const quietLog = new Proxy({}, { get: () => () => {} })

async function exists(path) {
  try {
    await access(path)
    return true
  } catch {
    return false
  }
}

async function prepareBase() {
  if (await exists(join(baseApp, 'node_modules'))) {
    execFileSync('pragma', ['upgrade'], { cwd: baseApp, stdio: 'ignore' })
    return
  }
  await mkdir(workRoot, { recursive: true })
  execFileSync('pragma', ['new', baseApp], { stdio: 'ignore' })
  execFileSync('npm', ['i', ...packages, '--ignore-scripts'], { cwd: baseApp, stdio: 'ignore' })
}

async function writeExample(app, files) {
  for (const file of files) {
    if (file.path.startsWith('dist/')) continue
    const target = join(app, file.path)
    const code = file.lines.map(line => line.text).join('\n') + '\n'
    await mkdir(dirname(target), { recursive: true })
    const appended = file.path === 'lib/theme.css' ? (await readFile(target, 'utf8')) + '\n' : ''
    await writeFile(target, appended + code)
  }
}

function checkApp(app) {
  try {
    execFileSync('pragma', ['check', '--strict'], { cwd: app, encoding: 'utf8', stdio: 'pipe' })
    return ''
  } catch (failure) {
    return failure.stdout + failure.stderr
  }
}

async function copyBinaryFiles(app, section) {
  const demo = join('routes/examples', section, 'demo')
  if (!(await exists(demo))) return
  for (const entry of await readdir(demo, { recursive: true })) {
    if (!entry.endsWith('.wasm')) continue
    await mkdir(dirname(join(app, 'routes', entry)), { recursive: true })
    await cp(join(demo, entry), join(app, 'routes', entry))
  }
}

await prepareBase()
let failing = 0
const only = process.argv.slice(2)
for (const [section, importSection] of sections) {
  if (only.length && !only.includes(section)) continue
  const { load } = await importSection()
  const { examples } = await load({ request: new Request('http://127.0.0.1:5181/'), log: quietLog, query: {}, params: {} })
  for (const [key, { files }] of Object.entries(examples)) {
    const id = section + '/' + key
    if (skipped.has(id)) continue
    const app = join(workRoot, section + '-' + key)
    await rm(app, { recursive: true, force: true })
    await cp(baseApp, app, { recursive: true })
    await writeExample(app, files)
    await copyBinaryFiles(app, section)
    const problems = checkApp(app)
    await rm(app, { recursive: true, force: true })
    if (problems) failing += 1
    process.stdout.write((problems ? 'FAIL ' : 'ok   ') + id + '\n' + problems)
  }
}
process.stdout.write(failing + ' failing\n')
process.exit(failing ? 1 : 0)
