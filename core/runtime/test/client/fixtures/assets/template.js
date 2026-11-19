// a top-level const holding the result of a call (like `debounce(save, 300)`) is a handler too
const twice = action => (...args) => { action(...args); action(...args) }

export default $ => ({
  name: 'Ana',
  id: 7,
  count: 2,
  log: '',
  tasks: [
    { id: 1, title: 'Buy <milk>', done: false },
    { id: 2, title: 'Walk', done: true },
  ],
  total: () => $.tasks.length,
  pending: () => $.tasks.filter(task => !task.done),
  toggle(task) {
    task.done = !task.done
  },
  rename(task, title) {
    task.title = title
  },
  record(event, label, amount, flag, nothing, name) {
    $.log = [event.type, label, amount, flag, nothing, name].join('|')
  },
  query: '',
  spot: '',
  search(event) {
    $.query = event.target.value
  },
  locate(el, event) {
    $.spot = el.id + ':' + event.type
  },
  saves: 0,
  saveTwice: twice(() => { $.saves += 1 }),
  mount() {
    self.__page = $
  },
})
