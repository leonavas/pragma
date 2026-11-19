import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/styling', {
    utilities: [
      '_components/build-card/build-card.html',
      '_snippets/utilities/rejected.html',
      '_snippets/utilities/check.txt',
    ],
    tokens: ['_snippets/tokens/theme.css', '_components/swatches/swatches.html'],
    scoped: ['_components/density/density.html', '_snippets/scoped/app.css'],
    dynamic: [
      '_components/build-status/build-status.html',
      '_snippets/build-status/rejected.html',
      '_snippets/build-status/check.txt',
    ],
    theme: ['_components/theme-preview/theme-preview.html', '_snippets/theme-switch/theme-switch.html'],
  })
  return { examples }
}
