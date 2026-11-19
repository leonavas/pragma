export const state = () => ({
  name: 'Ada',
  quantity: 2,
  shipping: 'standard',
  size: 'm',
  gift: false,
  note: '',
  get readout() {
    return ['name', 'quantity', 'shipping', 'size', 'gift', 'note'].map(name => ({
      name,
      shown: JSON.stringify(this[name]),
      kind: this[name] === null ? 'null' : typeof this[name],
    }))
  },
})
