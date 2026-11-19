import { store } from '../../core/state.js'

export default store({
  draft: '',
  pinned: false,
  get characters() { return `${this.draft.length} / 280` },
  get pinText() { return this.pinned ? 'Unpin' : 'Pin' },
  togglePin() { this.pinned = !this.pinned },
  wipe() { this.draft = '' },
}, { persist: 'scratchpad' })
