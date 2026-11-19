let draft = ''
let pinned = false
export const persist = ['draft', 'pinned']

const pinText = () => pinned ? 'Unpin' : 'Pin'

function togglePin() {
  pinned = !pinned
}

function wipe() {
  draft = ''
}
