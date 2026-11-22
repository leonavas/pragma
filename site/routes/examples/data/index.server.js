import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/data', {
    catalog: ['demo/catalog.html', 'demo/catalog.server.js'],
    failure: ['demo/catalog.server.js', 'demo/catalog.html'],
    guestbook: ['demo/guestbook.html', 'demo/guestbook.server.js'],
    clock: ['demo/clock.html', 'demo/clock.server.js'],
  })
  return { examples }
}
