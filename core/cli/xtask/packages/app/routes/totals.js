import groupBy from 'lodash/groupBy'
import sumBy from 'lodash/sumBy'

const stock = [
  { name: 'Coffee', kind: 'drink', units: 4 },
  { name: 'Tea', kind: 'drink', units: 6 },
  { name: 'Bread', kind: 'food', units: 5 },
]

export const state = () => ({
  totalStock: sumBy(stock, item => item.units),
  stockGroups: Object.keys(groupBy(stock, item => item.kind)).join(', '),
})
