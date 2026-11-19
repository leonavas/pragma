export const state = () => ({
  targets: [
    { id: 'alpha', name: 'alpha' },
    { id: 'beta', name: 'beta' },
  ],
  reading: [{ id: 'hint', field: 'ctx', value: 'click a button or type a key' }],
})

function describe(ctx, extra) {
  return [
    { id: 'event', field: 'ctx.event.type', value: ctx.event.type },
    { id: 'el', field: 'ctx.el', value: `${ctx.el.localName}[aria-label="${ctx.el.getAttribute('aria-label')}"]` },
    ...extra,
  ]
}

export const actions = {
  inspect(ctx) {
    ctx.$.reading = describe(ctx, [
      { id: 'scope', field: 'ctx.scope.target.name', value: ctx.scope.target.name },
      { id: 'detail', field: 'ctx.event.detail', value: String(ctx.event.detail) },
    ])
  },
  inspectKey(ctx) {
    ctx.$.reading = describe(ctx, [
      { id: 'key', field: 'ctx.event.key', value: ctx.event.key },
      { id: 'scope', field: 'ctx.scope', value: 'none outside a for loop' },
    ])
    ctx.log.debug('probe key', { 'key.name': ctx.event.key })
  },
}
