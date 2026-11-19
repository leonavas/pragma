const maxEntries = 5

function record(ctx, name, detail) {
  ctx.$.counter += 1
  ctx.$.entries = [{ id: ctx.$.counter, name, detail }, ...ctx.$.entries].slice(0, maxEntries)
}

export const state = () => ({
  draft: '',
  counter: 0,
  entries: [],
})

export const actions = {
  typed(ctx) { record(ctx, '@input', ctx.$.draft.length + ' chars') },
  pressed(ctx) {
    if (ctx.event.key !== 'Escape') return
    ctx.$.draft = ''
    record(ctx, '@keydown', 'Escape cleared the field')
  },
  left(ctx) { record(ctx, '@focusout', 'field lost focus') },
  doubled(ctx) { record(ctx, '@dblclick', 'on ' + ctx.el.localName) },
  save(ctx) {
    record(ctx, '@submit', 'saved "' + ctx.$.draft + '", no reload')
    ctx.$.draft = ''
  },
}
