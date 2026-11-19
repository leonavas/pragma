import { readExamples } from '../_source.server.js'

export const ssr = true

const maxEntries = 5
const entries = [{ id: 'seed', name: 'pragma', message: 'Signed from the server at boot.', at: '00:00' }]

function rejected(message) {
  return new Response(JSON.stringify({ error: { message } }), { status: 422, headers: { 'content-type': 'application/json' } })
}

export async function load() {
  const examples = await readExamples('routes/examples/data', {
    catalog: [
      'demo/catalog.html',
      'demo/catalog.js',
      'demo/catalog.server.js',
      'demo/_catalog.server.js',
      'demo/_products.json',
    ],
    guestbook: [
      '_components/guestbook/guestbook.html',
      '_components/guestbook/guestbook.js',
      'index.server.js',
    ],
    ssr: ['demo/ssr.server.js', 'demo/ssr.html', 'demo/ssr.js'],
    failure: ['demo/catalog.server.js', 'demo/catalog.html'],
  })
  return { examples, guestbook: entries }
}

export const actions = {
  async sign({ input, request, log }) {
    const name = String(input?.name ?? '').trim()
    const message = String(input?.message ?? '').trim()
    if (name.length < 2 || name.length > 40) return rejected('Name must be 2 to 40 characters.')
    if (message.length < 3 || message.length > 140) return rejected('Message must be 3 to 140 characters.')
    entries.unshift({ id: crypto.randomUUID(), name, message, at: new Date().toISOString().slice(11, 16) })
    entries.length = Math.min(entries.length, maxEntries)
    log.info('guestbook signed', { 'user_agent.original': request.headers.get('user-agent') ?? '' })
    return { guestbook: entries }
  },
}
