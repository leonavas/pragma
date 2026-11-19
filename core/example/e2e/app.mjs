import { spawn, execFileSync } from 'node:child_process'
import { cpSync, mkdtempSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const pw = createRequire(process.env.PLAYWRIGHT_DIR ? join(process.env.PLAYWRIGHT_DIR, '/') : import.meta.url)('playwright')
export const PRAGMA = process.env.PRAGMA ?? 'pragma'
export const browsers = (process.env.BROWSERS ?? 'chromium,firefox').split(',')
export const launch = name => pw[name].launch(name === 'chromium' ? { ignoreDefaultArgs: ['--hide-scrollbars'] } : {})

const source = fileURLToPath(new URL('..', import.meta.url))
const skip = new Set(['dist', '.pragma', 'e2e', 'node_modules'])

export function copyApp() {
  const dir = mkdtempSync(join(tmpdir(), 'minimart-'))
  cpSync(source, dir, { recursive: true, filter: f => !skip.has(f.slice(source.length).split('/')[0]) })
  return dir
}

export async function start(cmd, port) {
  const dir = copyApp()
  if (cmd === 'serve') execFileSync(PRAGMA, ['build'], { cwd: dir, stdio: 'ignore' })
  const proc = spawn(PRAGMA, [cmd, '--port', String(port)], { cwd: dir, stdio: ['ignore', 'pipe', 'pipe'] })
  let output = ''
  proc.stdout.on('data', chunk => { output += chunk })
  proc.stderr.on('data', chunk => { output += chunk })
  const url = `http://127.0.0.1:${port}`
  for (let i = 0; ; i++) {
    try {
      if ((await fetch(url + '/')).ok && (await fetch(url + '/_data/shop?category=x')).ok) break
    } catch {}
    if (i > 100) throw new Error(`${cmd} did not start:\n${output}`)
    await new Promise(resolve => setTimeout(resolve, 100))
  }
  return {
    url,
    dir,
    log: () => output,
    async stop() {
      proc.kill('SIGINT')
      await new Promise(resolve => proc.once('exit', resolve))
      rmSync(dir, { recursive: true, force: true })
    },
  }
}

export const timers = () => {
  const active = new Set()
  const setNative = self.setInterval, clearNative = self.clearInterval
  self.setInterval = (...args) => { const id = setNative(...args); active.add(id); return id }
  self.clearInterval = id => { active.delete(id); clearNative(id) }
  self.__timers = () => active.size
}
