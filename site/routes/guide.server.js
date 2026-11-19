import { readExamples } from './examples/_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes', {
    counter: ['_components/counter/counter.html'],
  })
  return { examples }
}
