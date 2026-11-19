import { readExamples } from '../_source.server.js'

export const ssr = true

const component = name => [`_components/${name}/${name}.html`, `_components/${name}/${name}.js`, `_components/${name}/${name}.css`]

export async function load() {
  const examples = await readExamples('routes/examples/syntax', {
    bindings: component('bindings'),
    classes: component('classes'),
    visibility: component('visibility'),
    model: component('model'),
    events: component('events'),
    overlay: ['_snippets/overlay-shortcut/usage.html', '_components/overlay-shortcut/overlay-shortcut.html'],
    conditions: component('conditions'),
    lists: component('lists'),
    badge: ['_snippets/badge/usage.html', '_components/badge/badge.html', '_components/badge/badge.css'],
    rejected: ['_snippets/rejected/wrong.html'],
  })
  return { examples }
}
