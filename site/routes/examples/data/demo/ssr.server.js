export const ssr = true

const votes = new Map([['prebuilt', 12], ['ssr', 7], ['islands', 3]])
const labels = { prebuilt: 'Prebuilt HTML + /_data fetch', ssr: 'ssr = true', islands: 'Client-only rendering' }

function tally() {
  return [...votes].map(([id, total]) => ({ id, label: labels[id], total }))
}

function sumOf(options) {
  return options.reduce((sum, option) => sum + option.total, 0)
}

export async function load({ request }) {
  const options = tally()
  return {
    options,
    votes: sumOf(options),
    renderedAt: new Date().toISOString().slice(11, 23),
    agent: (request.headers.get('user-agent') ?? 'unknown').split(' ')[0],
  }
}

export const actions = {
  async vote({ input, log }) {
    if (!votes.has(input?.id)) return new Response(JSON.stringify({ error: { message: 'Unknown option' } }), { status: 422 })
    votes.set(input.id, votes.get(input.id) + 1)
    log.info('ssr demo vote', { 'vote.option': input.id })
    const options = tally()
    return { options, votes: sumOf(options) }
  },
}
