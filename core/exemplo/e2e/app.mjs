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

// Cópia do app: os testes de dev editam arquivos sem tocar no repositório.
export function copyApp() {
  const dir = mkdtempSync(join(tmpdir(), 'mercadinho-'))
  cpSync(source, dir, { recursive: true, filter: f => !skip.has(f.slice(source.length).split('/')[0]) })
  return dir
}

export async function start(cmd, port) {
  const dir = copyApp()
  if (cmd === 'serve') execFileSync(PRAGMA, ['build'], { cwd: dir, stdio: 'ignore' })
  const proc = spawn(PRAGMA, [cmd, '--port', String(port)], { cwd: dir, stdio: ['ignore', 'pipe', 'pipe'] })
  let out = ''
  proc.stdout.on('data', d => { out += d })
  proc.stderr.on('data', d => { out += d })
  const url = `http://127.0.0.1:${port}`
  for (let i = 0; ; i++) {
    try {
      if ((await fetch(url + '/')).ok && (await fetch(url + '/_data/loja?categoria=x')).ok) break
    } catch {}
    if (i > 100) throw new Error(`${cmd} não subiu:\n${out}`)
    await new Promise(r => setTimeout(r, 100))
  }
  return {
    url,
    dir,
    log: () => out,
    async stop() {
      proc.kill('SIGINT')
      await new Promise(r => proc.once('exit', r))
      rmSync(dir, { recursive: true, force: true })
    },
  }
}

// Conta timers vivos para provar que o relógio para ao sair do Painel.
export const timers = () => {
  const vivos = new Set()
  const set = self.setInterval, clear = self.clearInterval
  self.setInterval = (...a) => { const id = set(...a); vivos.add(id); return id }
  self.clearInterval = id => { vivos.delete(id); clear(id) }
  self.__timers = () => vivos.size
}
