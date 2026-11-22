const words = ['Each', 'line', 'comes', 'alone.']
const wait = ms =>
  new Promise(done => setTimeout(done, ms))

export const actions = {
  async ask() {
    const encoder = new TextEncoder()
    const lines = new ReadableStream({
      async start(controller) {
        for (const word of words) {
          const line = JSON.stringify({ word }) + '\n'
          controller.enqueue(encoder.encode(line))
          await wait(300)
        }
        controller.close()
      },
    })
    const type = 'application/x-ndjson'
    return new Response(lines, {
      headers: { 'content-type': type },
    })
  },
}
