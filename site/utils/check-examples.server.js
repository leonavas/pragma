import { cp, mkdir, writeFile, readFile, rm, access } from 'node:fs/promises'
import { execFileSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'
import { load as syntaxLoad } from '../routes/examples/syntax/index.server.js'
import { load as componentsLoad } from '../routes/examples/components/index.server.js'
import { load as stateLoad } from '../routes/examples/state/index.server.js'
import { load as storesLoad } from '../routes/examples/stores/index.server.js'
import { load as routingLoad } from '../routes/examples/routing/index.server.js'
import { load as dataLoad } from '../routes/examples/data/index.server.js'
import { load as overlaysLoad } from '../routes/examples/overlays/index.server.js'
import { load as stylingLoad } from '../routes/examples/styling/index.server.js'
import { load as packagesLoad } from '../routes/examples/packages/index.server.js'
import { load as logsLoad } from '../routes/examples/logs/index.server.js'
import { load as seoLoad } from '../routes/examples/seo/index.server.js'

const sections = {
  syntax: syntaxLoad,
  components: componentsLoad,
  state: stateLoad,
  stores: storesLoad,
  routing: routingLoad,
  data: dataLoad,
  overlays: overlaysLoad,
  styling: stylingLoad,
  packages: packagesLoad,
  logs: logsLoad,
  seo: seoLoad,
}
const skipped = new Set(['syntax/rejected', 'state/mistakes', 'seo/rejected', 'seo/missing'])
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
  if (await exists(join(baseApp, 'node_modules'))) return
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

await prepareBase()
let failing = 0
for (const [section, load] of Object.entries(sections)) {
  const { examples } = await load({ request: new Request('http://127.0.0.1:5181/'), log: quietLog, query: {}, params: {} })
  for (const [key, { files }] of Object.entries(examples)) {
    const id = section + '/' + key
    if (skipped.has(id)) continue
    const app = join(workRoot, section + '-' + key)
    await rm(app, { recursive: true, force: true })
    await cp(baseApp, app, { recursive: true })
    await writeExample(app, files)
    const problems = checkApp(app)
    if (problems) failing += 1
    process.stdout.write((problems ? 'FAIL ' : 'ok   ') + id + '\n' + problems)
  }
}
process.stdout.write(failing + ' failing\n')
process.exit(failing ? 1 : 0)
