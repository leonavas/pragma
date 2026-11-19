import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/packages', {
    highlighter: ['../_source.server.js', '../../../utils/highlight.js', '../../../lib/components/showcase/showcase.js'],
    emailCheck: ['_components/email-check/email-check.html'],
    stockFilter: ['_components/stock-filter/stock-filter.html'],
    relativeTime: ['_components/relative-time/relative-time.html'],
  })
  return { examples }
}
