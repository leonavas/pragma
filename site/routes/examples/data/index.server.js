import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/data', {
    catalog: ['demo/catalog.html', 'demo/catalog.server.js'],
    failure: ['demo/catalog.server.js', 'demo/catalog.html'],
    clock: ['demo/clock.html', 'demo/clock.server.js'],
    visits: ['demo/visits.server.js', 'demo/visits.html'],
    guestbook: ['demo/guestbook.html', 'demo/guestbook.server.js'],
    booking: ['demo/booking.server.js', 'demo/booking.html'],
    redirects: ['demo/order.server.js', 'demo/done.server.js', 'demo/order.html', 'demo/done.html'],
    middleware: [{ file: '_snippets/middleware/middleware.js.txt', as: 'routes/_middleware.server.js' }],
    endpoint: ['demo/export.server.js', 'demo/downloads.html'],
    stream: ['demo/ask.server.js', 'demo/ask.html'],
  })
  return { examples }
}
