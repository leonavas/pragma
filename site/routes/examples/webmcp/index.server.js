import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/webmcp', {
    tool: ['_components/add-task/add-task.html'],
    reply: ['_components/book-table/book-table.html'],
    css: ['_components/find-note/find-note.html'],
    notool: ['_components/sign-in/sign-in.html'],
    rejected: ['_snippets/rejected/orders.html'],
    off: [{ file: '_snippets/off/pragma.json', as: 'pragma.json' }],
  })
  return { examples }
}
