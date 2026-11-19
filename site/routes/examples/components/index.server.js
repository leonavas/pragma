import { readExamples } from '../_source.server.js'
import { plugins } from './_plugins.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/components', {
    counter: [
      '_snippets/counter/usage.html',
      '_components/counter/counter.html',
      '_components/counter/counter.js',
      '_components/counter/counter.css',
    ],
    slots: [
      '_snippets/slots/usage.html',
      'index.js',
      '_components/panel/panel.html',
      '_components/panel/panel.js',
      '_components/panel/panel.css',
    ],
    'dynamic-props': [
      '_snippets/dynamic-props/usage.html',
      'index.js',
      '_plugins.server.js',
      '_components/search-field/search-field.html',
      '_components/plugin-row/plugin-row.html',
    ],
    badge: [
      '_snippets/badge/usage.html',
      '_components/badge/badge.html',
      '_components/badge/badge.css',
    ],
    'prop-probe': [
      '_snippets/prop-probe/usage.html',
      '_components/prop-probe/prop-probe.html',
      '_components/prop-probe/prop-probe.js',
    ],
  })
  return { examples, plugins }
}
