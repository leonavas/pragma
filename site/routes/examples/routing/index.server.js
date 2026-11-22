import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/routing', {
    files: ['demo/index.html', 'demo/about.html'],
    root: [{ file: '_snippets/root/layout.html', as: 'routes/_layout.html' }],
    param: ['demo/shop/index.html', 'demo/shop/[slug].html'],
    layout: ['demo/shop/_layout.html'],
    tabs: ['demo/shop/tabs.html'],
    active: ['demo/shop/sort.html'],
    navigate: ['demo/shop/go.html'],
  })
  return { examples }
}
