import { store } from '/core/state.js'

export default store({ n: 1, get total() { return this.n * 10 } }, { persist: 'account' })
