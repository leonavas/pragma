import { plainLines } from '../../../../../utils/code-lines.js'

function tokensOf(text, language, highlight) {
  const lines = highlight && language === 'html' ? highlight(text, language) : plainLines(text)
  return lines[0]?.tokens ?? []
}

export const state = ctx => ({
  selectedPath: '',
  highlight: null,
  get responses() {
    return ctx.data?.served?.[ctx.$.group] ?? []
  },
  get active() {
    return this.responses.find(response => response.path === this.selectedPath) ?? this.responses[0] ?? null
  },
  get hasChoice() {
    return this.responses.length > 1
  },
  get tabs() {
    return this.responses.map(response => ({
      path: response.path,
      label: response.path.replace('/examples/seo/demo', ''),
      pressed: response === this.active ? 'true' : 'false',
    }))
  },
  get lines() {
    const active = this.active
    if (!active) return []
    return active.lines.map((line, index) => ({
      id: active.path + '-' + index,
      origin: line.origin,
      tokens: tokensOf(line.text, active.language, this.highlight),
    }))
  },
  get hasLines() {
    return this.lines.length > 0
  },
  get statusText() {
    return this.active ? String(this.active.status || 'failed') : ''
  },
  get card() {
    return this.active?.card ?? null
  },
  get codeLabel() {
    return 'Response from ' + (this.active?.path ?? '')
  },
})

export const actions = {
  select(ctx) {
    ctx.$.selectedPath = ctx.scope.tab.path
  },
}

export function mount(ctx) {
  let mounted = true
  import('../../../../../utils/highlight.js').then(highlighter => {
    if (mounted) ctx.update(current => { current.highlight = highlighter.highlightLines })
  })
  return () => { mounted = false }
}
