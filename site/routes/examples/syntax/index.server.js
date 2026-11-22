import { readExamples } from '../_source.server.js'

export const ssr = true

const asPage = name => [
  { file: `_components/${name}/${name}.html`, as: 'routes/index.html' },
]

export async function load() {
  const examples = await readExamples('routes/examples/syntax', {
    greeting: asPage('greeting'),
    score: asPage('score'),
    nameField: asPage('name-field'),
    loudName: asPage('loud-name'),
    deployButton: asPage('deploy-button'),
    taskLine: asPage('task-line'),
    noteToggle: asPage('note-toggle'),
    orderStatus: asPage('order-status'),
    taskList: asPage('task-list'),
    rejected: ['_snippets/rejected/wrong.html'],
  })
  return { examples }
}
