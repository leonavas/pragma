import articles from './_articles.json'
  with { type: 'json' }

export const ssr = true

const notes = 'https://pragma.leonavas.com/examples/seo/demo/'

export async function load({ params }) {
  const article = articles.find(
    entry => entry.slug === params.slug,
  )
  if (!article) {
    return { title: 'Not found', robots: 'noindex' }
  }
  const url = notes + article.slug
  return { article, title: article.title, url }
}
