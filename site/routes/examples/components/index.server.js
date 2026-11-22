import { readExamples } from '../_source.server.js'

export const ssr = true

const page = name => ({ file: `_components/${name}/${name}.html`, as: 'routes/index.html' })

export async function load() {
  const examples = await readExamples('routes/examples/components', {
    counter: ['_components/counter/counter.html', '_snippets/counter/usage.html'],
    product: ['_components/product/product.html', '_snippets/product/usage.html'],
    teamList: ['_components/person-row/person-row.html', page('team-list')],
    slots: ['_components/panel/panel.html', page('release-notes')],
    slotOwner: ['_components/panel/panel.html', page('post')],
    badge: ['_components/badge/badge.html', '_snippets/badge/usage.html'],
    fileTree: ['_components/file-tree/file-tree.html', '_snippets/file-tree/usage.html', 'index.js'],
  })
  return { examples }
}
