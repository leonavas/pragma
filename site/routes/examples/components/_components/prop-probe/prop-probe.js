export const state = ctx => ({
  get jsTruthy() { return Boolean(ctx.$.locked) },
  get jsFalsy() { return !ctx.$.locked },
  get jsReading() { return JSON.stringify(ctx.$.locked) + (ctx.$.locked ? ' truthy' : ' falsy') },
})
