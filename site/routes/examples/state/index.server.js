import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/state', {
    page: ['index.js', '_snippets/page/usage.html'],
    split: ['_components/split/split.html'],
    queue: ['_components/queue/queue.html'],
    props: ['_components/usage/usage.html', '_snippets/usage/usage.html'],
    clock: ['_components/clock/clock.html'],
    stopwatch: ['_components/stopwatch/stopwatch.html'],
    probe: ['_components/probe/probe.html'],
  })
  return { examples }
}
