export const codeKinds = ['keyword', 'string', 'number', 'tag', 'attribute', 'title', 'comment', 'punctuation', 'directive', 'component']

export function plainLines(code) {
  return code.split('\n').map((text, index) => ({
    id: 'line-' + (index + 1),
    tokens: text ? [{ id: 'line-' + (index + 1) + '-0', text, kind: 'hl-plain' }] : [],
  }))
}
