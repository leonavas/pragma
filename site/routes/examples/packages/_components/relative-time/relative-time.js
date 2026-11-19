const releaseDates = [
  { version: 'v0.3.0', date: '2026-11-02' },
  { version: 'v0.2.0', date: '2026-08-14' },
  { version: 'v0.1.0', date: '2025-12-01' },
]

export const state = () => ({
  phase: 'idle',
  loadMs: 0,
  agoByVersion: {},
  get busy() { return this.phase === 'loading' },
  get buttonText() { return this.phase === 'ready' ? 'Recompute' : 'Load dayjs' },
  get releases() {
    return releaseDates.map(release => ({ ...release, ago: this.agoByVersion[release.version] ?? '—' }))
  },
  get status() {
    if (this.phase === 'loading') return 'Fetching the chunk...'
    if (this.phase === 'failed') return 'Could not load dayjs.'
    if (this.phase === 'ready') return 'dayjs arrived in ' + this.loadMs + ' ms, after first paint.'
    return 'dayjs is not loaded yet.'
  },
})

export const actions = {
  async loadRelativeTimes(ctx) {
    const startedAt = performance.now()
    ctx.$.phase = 'loading'
    try {
      const [{ default: dayjs }, { default: relativeTime }] = await Promise.all([
        import('dayjs'),
        import('dayjs/plugin/relativeTime'),
      ])
      dayjs.extend(relativeTime)
      ctx.update(current => {
        current.agoByVersion = Object.fromEntries(releaseDates.map(release => [release.version, dayjs(release.date).fromNow()]))
        current.loadMs = Math.round(performance.now() - startedAt)
        current.phase = 'ready'
      })
    } catch (failure) {
      ctx.log.warn('dayjs import failed', { 'exception.message': failure.message })
      ctx.update(current => { current.phase = 'failed' })
    }
  },
}
