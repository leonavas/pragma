import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/state', {
    page: ['_snippets/page/usage.html', 'index.js'],
    split: ['_components/split/split.html', '_components/split/split.js', '_components/split/split.css'],
    queue: ['_components/queue/queue.html', '_components/queue/queue.js'],
    clock: ['_components/clock/clock.js', '_components/clock/clock.html'],
    stopwatch: ['_components/stopwatch/stopwatch.js', '_components/stopwatch/stopwatch.html'],
    probe: ['_components/probe/probe.js', '_components/probe/probe.html'],
  })
  return { examples }
}
