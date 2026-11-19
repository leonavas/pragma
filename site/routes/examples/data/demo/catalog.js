export const state = ctx => ({
  get filterLink() {
    const category = ctx.query.category
    return {
      all: !category && !ctx.query.fail && 'true',
      hardware: category === 'hardware' && 'true',
      books: category === 'books' && 'true',
      broken: Boolean(ctx.query.fail) && 'true',
    }
  },
})
