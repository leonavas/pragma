export const state = () => ({
  draft: '',
  pendingOnly: false,
  nextId: 4,
  tasks: [
    { id: 1, title: 'Write the page in plain HTML', done: true },
    { id: 2, title: 'Bind state with :text and .done', done: false },
    { id: 3, title: 'Run pragma check --strict', done: false },
  ],
  get visible() {
    return this.pendingOnly ? this.tasks.filter(task => !task.done) : this.tasks
  },
  get summary() {
    const pending = this.tasks.filter(task => !task.done).length
    return pending + ' of ' + this.tasks.length + ' pending'
  },
  get filterText() {
    return this.pendingOnly ? 'Show all' : 'Hide done'
  },
  get filterPressed() {
    return this.pendingOnly ? 'true' : 'false'
  },
})

export const actions = {
  add(ctx) {
    const title = ctx.$.draft.trim()
    if (!title) return
    ctx.$.tasks.push({ id: ctx.$.nextId, title, done: false })
    ctx.$.nextId += 1
    ctx.$.draft = ''
  },
  toggle(ctx) {
    const task = ctx.$.tasks.find(entry => entry.id === ctx.scope.task.id)
    task.done = !task.done
  },
  toggleFilter(ctx) {
    ctx.$.pendingOnly = !ctx.$.pendingOnly
  },
}
