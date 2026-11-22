import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/performance', {
    chunks: [
      '_components/like-button/like-button.html',
      '_snippets/chunks/usage.html',
      '_snippets/chunks/blog.html',
      '_snippets/chunks/about.html',
    ],
    lazy: [
      '_snippets/lazy/report.html',
      { file: '_snippets/lazy/csv.js', as: 'utils/csv.js' },
    ],
  })
  return { examples }
}
