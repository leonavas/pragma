import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/routing', {
    files: ['demo/index.html', 'demo/about.html'],
    param: ['demo/shop/index.html', 'demo/shop/[slug].html'],
    layout: ['demo/shop/_layout.html'],
    tabs: ['demo/shop/tabs.html'],
    navigate: ['demo/shop/go.html'],
  })
  return { examples }
}
