import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/packages', {
    highlighter: ['../_source.server.js', '../../../utils/highlight.js', '../../../lib/components/showcase/showcase.js'],
    'email-check': ['_components/email-check/email-check.js', '_components/email-check/email-check.html'],
    'stock-filter': ['_components/stock-filter/stock-filter.js', '_components/stock-filter/stock-filter.html'],
    'relative-time': ['_components/relative-time/relative-time.js', '_components/relative-time/relative-time.html'],
  })
  return { examples }
}
