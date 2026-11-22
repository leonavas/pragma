import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/layout', {
    row: [{ file: '_components/toolbar/toolbar.html', as: 'routes/index.html' }],
    stack: [{ file: '_components/profile/profile.html', as: 'routes/index.html' }],
    grid: [{ file: '_components/mailbox/mailbox.html', as: 'routes/index.html' }],
    responsive: ['demo/index.html'],
    sticky: [{ file: '_components/contacts/contacts.html', as: 'routes/index.html' }],
    absolute: [{ file: '_components/avatar/avatar.html', as: 'routes/index.html' }],
  })
  return { examples }
}
