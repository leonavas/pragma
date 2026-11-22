import { readExamples } from '../_source.server.js'

export const ssr = true

const asPage = name => [
  { file: `_components/${name}/${name}.html`, as: 'routes/index.html' },
]

export async function load() {
  const examples = await readExamples('routes/examples/state', {
    shoppingList: asPage('shopping-list'),
    ticketTotal: asPage('ticket-total'),
    saveButton: asPage('save-button'),
    clock: asPage('clock'),
    uploadBar: asPage('upload-bar'),
    draftAutosave: asPage('draft-autosave'),
    mistakes: ['_snippets/mistakes/orders.html'],
  })
  return { examples }
}
