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

export function watchErrors(page, errors) {
  page.on('pageerror', error => errors.push(`pageerror: ${error.message}`))
  page.on('console', message => message.type() === 'error' && errors.push(`console: ${message.text()}`))
}

export const watchFirstPaint = () => {
  self.__firstH1 = null
  self.__removedH1 = 0
  self.__addedText = ''
  self.__skeletons = 0
  new MutationObserver(records => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node.nodeType !== 1) continue
        self.__firstH1 ??= node.matches('main h1') ? node : node.querySelector('main h1')
        self.__addedText += node.textContent
        if (node.matches('.skeleton') || node.querySelector('.skeleton')) self.__skeletons++
      }
      for (const node of record.removedNodes) {
        if (node.nodeType === 1 && (node.matches('h1') || node.querySelector('h1'))) self.__removedH1++
      }
    }
  }).observe(document, { childList: true, subtree: true })
}

export const audit = () => {
  const problems = []
  const text = el => el?.textContent.replace(/\s+/g, ' ').trim() ?? ''
  const nameOf = el =>
    el.getAttribute('aria-label')?.trim() ||
    (el.getAttribute('aria-labelledby') ?? '').split(/\s+/).map(id => text(document.getElementById(id))).join(' ').trim() ||
    [...(el.labels ?? [])].map(text).join(' ').trim() ||
    (el.matches('button, a, [role="tab"]') ? text(el) : '') ||
    el.getAttribute('title')?.trim() ||
    (el.matches('a') ? [...el.querySelectorAll('img[alt]')].map(img => img.alt).join(' ').trim() : '')
  const h1s = document.querySelectorAll('h1').length
  if (h1s !== 1) problems.push(`${h1s} h1 elements`)
  if (!document.documentElement.lang) problems.push('html without lang')
  if (!document.title) problems.push('empty title')
  if (document.querySelectorAll('main').length !== 1) problems.push('main landmark count is not 1')
  for (const img of document.querySelectorAll('img:not([alt])')) problems.push(`img without alt: ${img.src}`)
  for (const el of document.querySelectorAll('a[href], button, input, select, textarea, output[role], dialog, [role="group"], [role="tablist"], [role="tabpanel"], nav'))
    if (!nameOf(el)) problems.push(`no accessible name: ${el.outerHTML.slice(0, 100)}`)
  const ids = [...document.querySelectorAll('[id]')].map(el => el.id)
  for (const id of new Set(ids.filter((id, i) => ids.indexOf(id) !== i))) problems.push(`duplicate id: ${id}`)
  for (const el of document.querySelectorAll('[aria-describedby], [aria-labelledby]'))
    for (const id of `${el.getAttribute('aria-describedby') ?? ''} ${el.getAttribute('aria-labelledby') ?? ''}`.split(/\s+/).filter(Boolean))
      if (!document.getElementById(id)) problems.push(`aria reference to missing id: ${id}`)
  return problems
}

export const timers = () => {
  const active = new Set()
  const setNative = self.setInterval, clearNative = self.clearInterval
  self.setInterval = (...args) => { const id = setNative(...args); active.add(id); return id }
  self.clearInterval = id => { active.delete(id); clearNative(id) }
  self.__timers = () => active.size
}
