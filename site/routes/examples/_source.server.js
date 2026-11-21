import sources from '../../utils/sources.json' with { type: 'json' }
import { markLines } from '../../utils/highlight.js'

const languageByExtension = { html: 'html', xml: 'html', js: 'javascript', json: 'javascript', css: 'css' }
const phoneColumns = 36
const longSourceRows = 16

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

async function readSource(folder, entry, example, index) {
  const path = joinPath(folder, entry)
  const raw = sources[path]
  if (raw === undefined) throw new Error('missing bundled source: ' + path)
  const code = raw.trimEnd().replaceAll('\t', '  ')
  const lines = markLines(code, languageOf(entry))
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
