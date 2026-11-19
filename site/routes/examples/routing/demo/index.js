import { posts } from './_posts.js'

export const state = () => ({
  posts: posts.map(post => ({ ...post, href: '/examples/routing/demo/' + post.slug })),
})
