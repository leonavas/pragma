import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const [section, site] = await Promise.all([
    readExamples('routes/examples/routing', {
      files: ['demo/index.html', 'demo/index.js', 'demo/new.html', 'demo/_posts.js'],
      param: ['demo/[slug].html', 'demo/[slug].js', 'demo/_posts.js'],
      local: ['demo/_components/post-meta/post-meta.html', 'demo/_components/demo-nav/demo-nav.html', 'demo/_components/demo-nav/demo-nav.css'],
      tabs: ['demo/tabs.html', 'demo/_components/query-tabs/query-tabs.html', 'demo/_components/query-tabs/query-tabs.js', 'demo/_components/query-tabs/query-tabs.css'],
      navigate: ['demo/go.html', 'demo/go.js'],
    }),
    readExamples('routes', {
      layout: ['_layout.html'],
      missing: ['_404.html'],
    }),
  ])
  return { examples: { ...section, ...site } }
}
