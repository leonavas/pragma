const sections = [
  { id: 'overview', label: 'Overview', body: 'A ten-key keyboard with hot-swap switches.' },
  { id: 'specs', label: 'Specs', body: '87 keys, USB-C, 1.2 kg, aluminium case.' },
  { id: 'reviews', label: 'Reviews', body: '4.8 out of 5 from 312 reviews.' },
]

export const state = ctx => ({
  clicks: 0,
  get active() { return sections.find(section => section.id === ctx.query.tab) ?? sections[0] },
  get tabs() {
    return sections.map(section => ({
      ...section,
      href: '?tab=' + section.id,
      current: section === this.active && 'true',
    }))
  },
  get body() { return this.active.body },
  get queryText() { return ctx.query.tab ?? 'undefined' },
  get bumpLabel() { return 'Clicked ' + this.clicks + ' times' },
})

export const actions = {
  bump(ctx) { ctx.$.clicks += 1 },
}
