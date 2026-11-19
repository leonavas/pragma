import { posts } from './_posts.js'

export const state = ctx => ({
  get post() { return posts.find(post => post.slug === ctx.params.slug) ?? null },
})
