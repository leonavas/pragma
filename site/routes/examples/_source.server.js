import { readFile } from 'node:fs/promises'
import { join, basename, dirname } from 'node:path'
import { markLines } from '../../utils/highlight.js'

const languageByExtension = { html: 'html', xml: 'html', js: 'javascript', json: 'javascript', css: 'css' }
const phoneColumns = 36
const longSourceRows = 16

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
  const path = join(folder, entry)
  const code = (await readFile(join(process.cwd(), path), 'utf8')).trimEnd().replaceAll('\t', '  ')
  const lines = markLines(code, languageOf(entry))
  return {
    id: 'source-' + example + '-' + index,
    name: basename(path),
    dir: dirname(path) + '/',
    path,
    lines,
    long: phoneRows(lines) > longSourceRows,
    active: index === 0,
    pressed: index === 0 ? 'true' : 'false',
    copied: false,
    blocked: false,
  }
}

export async function readExamples(folder, examples) {
  const entries = await Promise.all(Object.entries(examples).map(async ([example, sources]) => {
    const files = await Promise.all(sources.map((entry, index) => readSource(folder, entry, example, index)))
    return [camelCase(example), { files }]
  }))
  return Object.fromEntries(entries)
}
