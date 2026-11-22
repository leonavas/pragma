import sources from '../../utils/sources.json' with { type: 'json' }
import { markLines } from '../../utils/highlight.js'

const languageByExtension = { html: 'html', xml: 'html', js: 'javascript', json: 'javascript', css: 'css' }
const phoneColumns = 36
const longSourceRows = 16
const demoOnlyTag = /^<meta name="robots" content="noindex">\n/m

function joinPath(folder, entry) {
  const stack = []
  for (const part of (folder + '/' + entry).split('/')) {
    if (!part || part === '.') continue
    if (part === '..') stack.pop()
    else stack.push(part)
  }
  return stack.join('/')
}

function baseName(path) {
  return path.slice(path.lastIndexOf('/') + 1)
}

function dirName(path) {
  const index = path.lastIndexOf('/')
  return index === -1 ? '.' : path.slice(0, index)
}

function camelCase(name) {
  return name.replace(/-(\w)/g, (dash, letter) => letter.toUpperCase())
}

function phoneRows(lines) {
  return lines.reduce((rows, line) => rows + Math.max(1, Math.ceil(line.text.length / phoneColumns)), 0)
}

function languageOf(fileName) {
  return languageByExtension[fileName.split('.').pop()] ?? 'text'
}

function appPath(folder, path) {
  if (!path.startsWith(folder + '/')) return path
  const rest = path.slice(folder.length + 1)
  if (rest.startsWith('demo/')) return 'routes/' + rest.slice('demo/'.length)
  const snippet = rest.match(/^_snippets\/[^/]+\/(.+)$/)
  if (!snippet) return 'routes/' + rest
  return 'routes/' + (snippet[1] === 'usage.html' ? 'index.html' : snippet[1])
}

function appCode(folder, sourcePath, code) {
  const demoUrl = new RegExp('/examples/' + baseName(folder) + '/demo(/|(?![\\w-]))', 'g')
  if (!sourcePath.startsWith(folder + '/')) return code
  const isDemo = sourcePath.startsWith(folder + '/demo/')
  const visible = isDemo ? code.replace(demoOnlyTag, '') : code
  return visible.replace(demoUrl, '/')
}

async function readSource(folder, entry, example, index) {
  const file = typeof entry === 'string' ? entry : entry.file
  const sourcePath = joinPath(folder, file)
  const raw = sources[sourcePath]
  if (raw === undefined) throw new Error('missing bundled source: ' + sourcePath)
  const code = appCode(folder, sourcePath, raw.trimEnd().replaceAll('\t', '  '))
  const path = entry.as ?? appPath(folder, sourcePath)
  const lines = markLines(code, languageOf(file))
  return {
    id: 'source-' + example + '-' + index,
    name: baseName(path),
    dir: dirName(path) + '/',
    path,
    lines,
    long: phoneRows(lines) > longSourceRows,
    active: index === 0,
    expanded: false,
  }
}

export async function readExamples(folder, examples) {
  const entries = await Promise.all(Object.entries(examples).map(async ([example, sources]) => {
    const files = await Promise.all(sources.map((entry, index) => readSource(folder, entry, example, index)))
    return [camelCase(example), { files }]
  }))
  return Object.fromEntries(entries)
}
