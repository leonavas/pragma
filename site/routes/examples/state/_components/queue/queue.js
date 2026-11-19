const saveDelayMs = 900

export const state = () => ({
  draft: '',
  saving: false,
  savedAt: '',
  jobs: [
    { id: 'build', title: 'pragma build', done: true },
    { id: 'check', title: 'pragma check --strict', done: false },
  ],
  get pending() { return this.jobs.filter(job => !job.done).length },
  get summary() { return `${this.pending} of ${this.jobs.length} pending` },
  get saveText() { return this.saving ? 'Saving…' : 'Save' },
  get blank() { return !this.draft.trim() },
})

export const actions = {
  add(ctx) {
    if (ctx.$.blank) return
    ctx.$.jobs.push({ id: crypto.randomUUID(), title: ctx.$.draft.trim(), done: false })
    ctx.$.draft = ''
  },
  toggle(ctx) {
    ctx.scope.job.done = !ctx.scope.job.done
  },
  remove(ctx) {
    ctx.$.jobs = ctx.$.jobs.filter(job => job.id !== ctx.scope.job.id)
  },
  async save(ctx) {
    ctx.$.saving = true
    await new Promise(resolve => setTimeout(resolve, saveDelayMs))
    ctx.$.saving = false
    ctx.$.savedAt = new Date().toLocaleTimeString('en-GB')
  },
}
