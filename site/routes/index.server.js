import { readExamples } from './examples/_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes', {
    tasks: ['_components/task-list/task-list.html'],
  })
  return { examples }
}
