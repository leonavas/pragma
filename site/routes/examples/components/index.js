import { data } from 'pragma'

let likes = 0
let query = ''

const likesText = () => (likes === 1 ? '1 like' : likes + ' likes')

function visiblePlugins() {
  const needle = query.trim().toLowerCase()
  return (data?.plugins ?? []).filter(plugin => plugin.name.includes(needle) || plugin.kind.includes(needle))
}

function like() {
  likes += 1
}
