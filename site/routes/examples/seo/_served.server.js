import { readFile } from 'node:fs/promises'
import { join } from 'node:path'

const headPattern = /<head>([\s\S]*?)<\/head>/
const headTagPattern =/<title\b[^>]*>[^<]*<\/title>|<(?:meta|link)\b[^>]*>/g
const attributePattern = /([^\s=]+)="([^"]*)"/g
const plumbingPattern = /\bcharset=|name="viewport"|rel="(?:preload|modulepreload|stylesheet)"/
const entities = { '&amp;': '&', '&quot;': '"', '&#39;': "'", '&lt;': '<', '&gt;': '>' }
const maxBodyLines = 40
const missingFile = { tone: 'warn', note: 'Not in public/ for this deploy, so the server answers with the 404 page.' }

function decode(text) {
  return text.replace(/&(?:amp|quot|#39|lt|gt);/g, entity => entities[entity])
}

function attributesOf(tag) {
  return Object.fromEntries([...tag.matchAll(attributePattern)].map(([, name, value]) => [name, decode(value)]))
}

function keyOf(tag) {
  if (tag.startsWith('<title')) return 'title'
  const attributes = attributesOf(tag)
  if (tag.startsWith('<link')) return 'link ' + attributes.rel + ' ' + (attributes.hreflang ?? '')
  return 'meta ' + (attributes.name ?? attributes.property ?? attributes['http-equiv'])
}

function headTagsOf(html) {
  const head = html.match(headPattern)?.[1] ?? ''
  return (head.match(headTagPattern) ?? []).filter(tag => !plumbingPattern.test(tag))
}

function valuesOf(tags) {
  const values = {}
  for (const tag of tags) {
    const attributes = attributesOf(tag)
    if (tag.startsWith('<title')) values.title = decode(tag.slice(tag.indexOf('>') + 1, tag.lastIndexOf('<')))
    else if (tag.startsWith('<link')) values[attributes.rel] = attributes.href
    else values[attributes.name ?? attributes.property] = attributes.content
  }
  return values
}

function cardOf(values) {
  const image = values['twitter:image'] ?? values['og:image']
  const pageUrl = values['og:url'] ?? values.canonical
  return {
    host: pageUrl ? new URL(pageUrl).host : 'no og:url',
    title: values['twitter:title'] ?? values['og:title'] ?? values.title ?? '',
    description: values['twitter:description'] ?? values['og:description'] ?? values.description ?? '',
    imagePath: image ? new URL(image, 'https://pragma.dev').pathname : '',
    imageAlt: values['og:image:alt'] ?? '',
  }
}

function verdictOf(status, values) {
  if (status === 404) return { tone: 'warn', note: 'Real 404: no route matches, so the server sends routes/_404.html with status 404.' }
  if (status !== 200) return { tone: 'error', note: 'Unexpected status ' + status + '.' }
  if (values.robots?.includes('noindex')) return { tone: 'warn', note: 'Soft 404: status 200, but robots noindex keeps it out of search results.' }
  return { tone: 'ok', note: '' }
}

async function pageTagKeys(source) {
  if (!source) return new Set()
  const fragment = await readFile(join(process.cwd(), source), 'utf8')
  return new Set((fragment.match(headTagPattern) ?? []).map(keyOf))
}

export async function fetchServed(origin, { path, source }, log) {
  try {
    const [response, pageKeys] = await Promise.all([
      fetch(origin + path, { headers: { accept: 'text/html' }, redirect: 'manual' }),
      pageTagKeys(source),
    ])
    const body = await response.text()
    const type = response.headers.get('content-type')?.split(';')[0] ?? ''
    const html = type === 'text/html'
    const tags = html ? headTagsOf(body) : []
    const values = valuesOf(tags)
    const lines = html
      ? tags.map(tag => ({ text: tag, origin: pageKeys.has(keyOf(tag)) ? 'page' : 'layout' }))
      : body.trimEnd().split('\n').slice(0, maxBodyLines).map(text => ({ text, origin: '' }))
    return {
      path,
      status: response.status,
      type,
      language: html || type.endsWith('xml') ? 'html' : 'text',
      lines: source || response.status === 200 ? lines : [],
      card: html && response.status === 200 && !values.robots ? cardOf(values) : null,
      ...(source || response.status === 200 ? verdictOf(response.status, values) : missingFile),
    }
  } catch (failure) {
    log.warn('served page fetch failed', { 'url.path': path, 'exception.message': failure.message })
    return { path, status: 0, type: '', language: 'text', lines: [], card: null, tone: 'error', note: 'Request failed: ' + failure.message }
  }
}
