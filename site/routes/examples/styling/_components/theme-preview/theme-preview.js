export const state = () => ({
  dark: false,
  get mode() { return this.dark ? 'dark' : 'light' },
  get modeText() { return 'data-theme="' + this.mode + '"' },
  get pressed() { return String(this.dark) },
})

export const actions = {
  toggle(ctx) { ctx.$.dark = !ctx.$.dark },
}
