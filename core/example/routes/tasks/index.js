import { contains, markCurrent } from '../../utils/text.js'

const FILTERS = { all: () => true, pending: task => !task.done, completed: task => task.done }

function validate(tasks, title, id) {
  if (title.length < 3) return 'Use at least 3 characters.'
  if (title.length > 80) return 'Use at most 80 characters.'
  if (tasks.exists(title, id)) return 'This task already exists.'
  return ''
}

export const state = ({ query, shared }) => ({
  newTitle: '',
  error: '',
  search: '',
  editing: null,
  draft: '',
  get invalid() { return String(!!this.error) },
  get tone() { return shared.tasks.completed && !shared.tasks.pending ? 'ok' : 'neutral' },
  get filter() { return markCurrent(Object.keys(FILTERS), FILTERS[query.filter] ? query.filter : 'all') },
  get visible() {
    const matchesFilter = FILTERS[query.filter] ?? FILTERS.all
    return shared.tasks.items
      .filter(task => matchesFilter(task) && contains(task.title, this.search))
      .map(task => ({ ...task, editing: task.id === this.editing, removeLabel: `Remove ${task.title}` }))
  },
})

export const actions = {
  add(ctx) {
    const title = ctx.$.newTitle.trim()
    ctx.$.error = validate(ctx.shared.tasks, title)
    if (ctx.$.error) return
    ctx.shared.tasks.add(title)
    ctx.$.newTitle = ''
    ctx.el.elements.title.focus()
  },
  toggle(ctx) {
    const task = ctx.shared.tasks.find(ctx.scope.task.id)
    task.done = !task.done
  },
  remove(ctx) { ctx.shared.tasks.remove(ctx.scope.task.id) },
  edit(ctx) {
    ctx.$.editing = ctx.scope.task.id
    ctx.$.draft = ctx.scope.task.title
  },
  editKeydown(ctx) {
    if (ctx.event.key === 'Escape') ctx.$.editing = null
    if (ctx.event.key === 'Enter') saveDraft(ctx)
  },
  saveEdit(ctx) { saveDraft(ctx) },
  completeAll(ctx) {
    for (const task of ctx.shared.tasks.items) task.done = true
  },
  async clearCompleted(ctx) {
    if (await ctx.confirm('clear-completed') !== 'ok') return
    ctx.shared.tasks.clearCompleted()
    ctx.log.info('completed tasks removed')
  },
}

function saveDraft(ctx) {
  const id = ctx.$.editing, title = ctx.$.draft.trim()
  if (!id) return
  ctx.$.editing = null
  if (!validate(ctx.shared.tasks, title, id)) ctx.shared.tasks.find(id).title = title
}
