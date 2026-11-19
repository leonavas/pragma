import { store } from '../../core/state.js'

const createTask = title => ({ id: crypto.randomUUID(), title, done: false })
const sameTitle = (left, right) => left.localeCompare(right, 'en', { sensitivity: 'accent' }) === 0

export default store({
  items: [createTask('Buy coffee'), { ...createTask('Pay bills'), done: true }, createTask('Call the pharmacy')],
  get pending() { return this.items.filter(task => !task.done).length },
  get completed() { return this.items.length - this.pending },
  get summary() { return `${this.completed} of ${this.items.length}` },
  exists(title, exceptId) { return this.items.some(task => task.id !== exceptId && sameTitle(task.title, title)) },
  add(title) { this.items.push(createTask(title)) },
  find(id) { return this.items.find(task => task.id === id) },
  remove(id) { this.items = this.items.filter(task => task.id !== id) },
  clearCompleted() { this.items = this.items.filter(task => !task.done) },
}, { persist: 'tasks' })
