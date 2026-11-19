const createTask = title => ({ id: crypto.randomUUID(), title, done: false })
const sameTitle = (left, right) => left.localeCompare(right, 'en', { sensitivity: 'accent' }) === 0

let items = [createTask('Buy coffee'), { ...createTask('Pay bills'), done: true }, createTask('Call the pharmacy')]
export const persist = ['items']

const pending = () => items.filter(task => !task.done).length
const completed = () => items.length - pending()
const summary = () => `${completed()} of ${items.length}`

function exists(title, exceptId) {
  return items.some(task => task.id !== exceptId && sameTitle(task.title, title))
}

function find(id) {
  return items.find(task => task.id === id)
}

function add(title) {
  items.push(createTask(title))
}

function remove(id) {
  items = items.filter(task => task.id !== id)
}

function completeAll() {
  for (const task of items) task.done = true
}

function clearCompleted() {
  items = items.filter(task => !task.done)
}
