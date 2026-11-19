import articles from './_articles.json' with { type: 'json' }

export const ssr = true

const siteOrigin = 'https://pragma.dev'

function headFor(article) {
  if (!article) return { title: 'Note not found · pragma field notes', robots: 'noindex' }
  return {
    title: article.title + ' · pragma field notes',
    description: article.summary,
    url: siteOrigin + '/examples/seo/demo/' + article.slug,
    image: siteOrigin + article.image,
    imageAlt: article.imageAlt,
    published: article.published,
  }
}

export async function load({ params }) {
  const article = articles.find(entry => entry.slug === params.slug) ?? null
  return { article, head: headFor(article) }
}
