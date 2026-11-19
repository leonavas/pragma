import highlighter from 'highlight.js/lib/core'
import xml from 'highlight.js/lib/languages/xml'
import javascript from 'highlight.js/lib/languages/javascript'
import css from 'highlight.js/lib/languages/css'
import { plainLines, codeKinds } from './code-lines.js'

highlighter.registerLanguage('html', xml)
highlighter.registerLanguage('javascript', javascript)
highlighter.registerLanguage('css', css)

const cache = new Map()
const sigils = new Set(['@', '$', '.'])
const blockTags = new Set(['if', 'else-if', 'else', 'for', 'empty'])
const maxIndent = 16
const kindOfScope = {
  'hl-keyword': 'keyword', 'hl-built_in': 'keyword', 'hl-literal': 'keyword',
  'hl-string': 'string', 'hl-regexp': 'string', 'hl-symbol': 'string',
  'hl-number': 'number',
  'hl-name': 'tag', 'hl-selector-tag': 'tag', 'hl-selector-class': 'tag', 'hl-selector-id': 'tag', 'hl-selector-pseudo': 'tag',
  'hl-attr': 'attribute', 'hl-attribute': 'attribute', 'hl-property': 'attribute',
  'hl-title': 'title', 'hl-variable': 'title', 'hl-params': 'title',
  'hl-comment': 'comment', 'hl-meta': 'comment',
  'hl-tag': 'punctuation', 'hl-punctuation': 'punctuation', 'hl-operator': 'punctuation',
  'hl-directive': 'directive', 'hl-block': 'directive',
  'hl-component': 'component',
}

function refineKind(kind, text) {
  if (kind === 'hl-attr' && /^[:@.]/.test(text)) return 'hl-directive'
  if (kind === 'hl-name' && blockTags.has(text)) return 'hl-block'
  if (kind === 'hl-name' && /^[cp]-/.test(text)) return 'hl-component'
  return kind
}

function scopeClass(scope) {
  return scope ? 'hl-' + scope.split('.')[0].replace('language:', 'lang-') : 'hl-plain'
}

function collectTokens(node, inheritedScope, pushText) {
  if (typeof node === 'string') return pushText(node, inheritedScope)
  const scope = node.scope && !node.scope.startsWith('language:') ? node.scope : inheritedScope
  for (const child of node.children) collectTokens(child, scope, pushText)
}

function splitIntoLines(rootNode) {
  const lines = [{ id: 'line-1', tokens: [] }]
  const pushText = (text, scope) => {
    text.split('\n').forEach((segment, index) => {
      if (index > 0) lines.push({ id: 'line-' + (lines.length + 1), tokens: [] })
      if (segment) pushToken(lines.at(-1), segment, scopeClass(scope))
    })
  }
  collectTokens(rootNode, '', pushText)
  return lines
}

function pushToken(line, text, kind) {
  const previous = line.tokens.at(-1)
  const sigil = previous?.text.slice(-1)
  if (kind === 'hl-attr' && sigils.has(sigil)) {
    previous.text = previous.text.slice(0, -1)
    if (!previous.text) line.tokens.pop()
    text = sigil + text
  }
  kind = refineKind(kind, text)
  const last = line.tokens.at(-1)
  if (last?.kind === kind) last.text += text
  else line.tokens.push({ id: line.id + '-' + line.tokens.length, text, kind })
}

export function highlightLines(code, language) {
  const cacheKey = language + '\n' + code
  if (cache.has(cacheKey)) return cache.get(cacheKey)
  const result = highlighter.getLanguage(language) ? highlighter.highlight(code, { language, ignoreIllegals: true }) : null
  const lines = result ? splitIntoLines(result._emitter.rootNode) : plainLines(code)
  cache.set(cacheKey, lines)
  return lines
}

function marksOf(tokens) {
  const marks = []
  let column = 0
  for (const token of tokens) {
    const kind = codeKinds.indexOf(kindOfScope[token.kind])
    if (kind >= 0 && token.text.trim()) marks.push(column, column + token.text.length, kind)
    column += token.text.length
  }
  return marks
}

export function markLines(code, language) {
  return highlightLines(code, language).map((line, index) => {
    const text = line.tokens.map(token => token.text).join('')
    const spaces = Math.min(text.length - text.trimStart().length, maxIndent)
    const indent = spaces - (spaces % 2)
    return { n: index + 1, text, indent: text.trim() && indent ? indent : null, marks: marksOf(line.tokens) }
  })
}
