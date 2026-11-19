import { readExamples } from '../_source.server.js'
import { plugins } from './_plugins.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/components', {
    counter: ['_components/counter/counter.html', '_snippets/counter/usage.html'],
    slots: ['_snippets/slots/usage.html', 'index.js', '_components/panel/panel.html'],
    'dynamic-props': [
      '_snippets/dynamic-props/usage.html',
      'index.js',
      '_plugins.server.js',
      '_components/search-field/search-field.html',
      '_components/plugin-row/plugin-row.html',
    ],
    badge: ['_snippets/badge/usage.html', '_components/badge/badge.html'],
    'prop-probe': ['_components/prop-probe/prop-probe.html', '_snippets/prop-probe/usage.html'],
  })
  return { examples, plugins }
}
