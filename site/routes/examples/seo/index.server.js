import { readExamples } from '../_source.server.js'
import { fetchServed } from './_served.server.js'

export const ssr = true

const article = 'routes/examples/seo/demo/[slug].html'

const requests = {
  static: [{ path: '/examples/seo/demo', source: 'routes/examples/seo/demo/index.html' }],
  dynamic: [
    { path: '/examples/seo/demo/prebuilt-html', source: article },
    { path: '/examples/seo/demo/head-tags-from-data', source: article },
  ],
  missing: [
    { path: '/examples/seo/demo/no-such-note', source: article },
    { path: '/examples/seo/demo/no/such/page', source: 'routes/_404.html' },
  ],
  files: [{ path: '/robots.txt' }, { path: '/sitemap.xml' }],
}

async function fetchGroups(origin, log) {
  const groups = await Promise.all(Object.entries(requests).map(async ([group, targets]) => {
    return [group, await Promise.all(targets.map(target => fetchServed(origin, target, log)))]
  }))
  return Object.fromEntries(groups)
}

export async function load({ request, log }) {
  const [examples, served] = await Promise.all([
    readExamples('routes/examples/seo', {
      static: ['demo/index.html'],
      dynamic: ['demo/[slug].html', 'demo/[slug].server.js', 'demo/_articles.json'],
      missing: ['demo/[slug].server.js'],
      files: ['../../../public/robots.txt', { file: '_snippets/files/sitemap.xml', as: 'public/sitemap.xml' }],
      rejected: ['_snippets/rejected/post.html'],
    }),
    fetchGroups(new URL(request.url).origin, log),
  ])
  return { examples, served }
}
