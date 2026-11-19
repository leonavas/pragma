const timeFormat = new Intl.DateTimeFormat('en-GB', { hour: '2-digit', minute: '2-digit', second: '2-digit' })

export const state = () => ({
  now: Date.now(),
  viewportWidth: window.innerWidth,
  resizes: 0,
  get timeLabel() { return timeFormat.format(this.now) },
  get widthLabel() { return `${this.viewportWidth}px` },
})

export function mount(ctx) {
  const timer = setInterval(() => ctx.update(current => { current.now = Date.now() }), 1000)
  const onResize = () => ctx.update(current => {
    current.viewportWidth = window.innerWidth
    current.resizes += 1
  })
  window.addEventListener('resize', onResize)
  return () => {
    clearInterval(timer)
    window.removeEventListener('resize', onResize)
  }
}
