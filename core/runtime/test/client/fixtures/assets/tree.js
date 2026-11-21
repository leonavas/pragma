const leaf = (id, depth) => ({ id, name: id.toUpperCase(), depth, children: [] })

export default $ => ({
  count: 1,
  left: 25,
  tone: 'red',
  rows: [{ id: 1, x: 10 }, { id: 2, x: null }],
  root: {
    id: 'root',
    name: 'root',
    children: [
      { ...leaf('a', 1), children: [leaf('a1', 2)] },
      leaf('b', 1),
    ],
  },
  mount() {
    self.__page = $
  },
})
