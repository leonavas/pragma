import { readExamples } from './examples/_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes', {
    tasks: ['_components/task-list/task-list.html', '_components/task-list/task-list.js'],
  })
  return { examples }
}
