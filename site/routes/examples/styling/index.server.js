import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/styling', {
    utilities: [{ file: '_components/hover-button/hover-button.html', as: 'routes/index.html' }],
    tokens: [
      { file: '_snippets/tokens/theme.css', as: 'lib/theme.css' },
      { file: '_components/swatch/swatch.html', as: 'routes/index.html' },
    ],
    scoped: [
      '_components/note/note.html',
      { file: '_snippets/scoped/app.css', as: 'dist/client/assets/css/index.css' },
    ],
    variables: [{ file: '_components/upload-bar/upload-bar.html', as: 'routes/index.html' }],
    theme: [
      '_components/invoice-card/invoice-card.html',
      '_snippets/theme/usage.html',
    ],
  })
  return { examples }
}
