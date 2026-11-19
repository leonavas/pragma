export const state = ctx => ({
  visitor: '',
  ownerTag: `${ctx.el.localName}[data-page]`,
  get greeting() { return this.visitor.trim() ? `Hello, ${this.visitor.trim()}.` : 'Hello, whoever you are.' },
  get anonymous() { return !this.visitor.trim() },
})

export const actions = {
  forget(ctx) { ctx.$.visitor = '' },
}
