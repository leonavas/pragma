import groupBy from 'lodash/groupBy'

const fruits = ['apple', 'avocado', 'grape', 'banana']

export const state = () => ({
  get fruitsByInitial() {
    return Object.entries(groupBy(fruits, fruit => fruit[0]))
      .map(([initial, inGroup]) => initial + ': ' + inGroup.length)
      .join(', ')
  },
})
