import { readExamples } from '../_source.server.js'
import { sizes } from './_sizes.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/packages', {
    signupEmail: [{ file: '_components/signup-email/signup-email.html', as: 'routes/index.html' }],
    releaseAge: [
      { file: '_components/release-age/release-age.html', as: 'routes/index.html' },
      { file: '_snippets/long-date/long-date.js', as: 'utils/long-date.js' },
    ],
    sum: ['demo/sum.html', 'demo/sum.server.js'],
  })
  return { examples, sizes }
}
