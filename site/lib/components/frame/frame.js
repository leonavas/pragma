const pollMs = 400
const clickWindowMs = 4000

function addressOf(view) {
  try {
    return view.location.protocol === 'about:' ? '' : view.location.pathname + view.location.search + view.location.hash
  } catch {
    return ''
  }
}

function loadMs(view) {
  const [entry] = view.performance.getEntriesByType('navigation')
  return entry ? Math.round(entry.loadEventStart || entry.domContentLoadedEventEnd) : 0
}

export const state = ctx => ({
  current: ctx.$.src,
  live: true,
  loading: false,
  navigation: '',
  timing: '',
  beat: '',
  get reloadLabel() {
    return 'Reload ' + this.current
  },
  get openLabel() {
    return 'Open ' + this.current + ' in a new tab'
  },
})

export const actions = {
  reload(ctx) {
    const view = ctx.el.closest('[data-c="frame"]').querySelector('iframe').contentWindow
    ctx.$.loading = true
    view.location.reload()
  },
}

export function mount(ctx) {
  const frame = ctx.el.querySelector('iframe')
  let view = null
  let startedAt = 0
  let poll = 0
  let swaps = null

  const show = (address, navigation, ms) => ctx.update(current => {
    current.loading = false
    current.navigation = navigation
    current.timing = navigation && ms > 0 ? ms + ' ms' : ''
    if (!address || current.current === address) return
    current.current = address
    current.beat = current.beat === 'a' ? 'b' : 'a'
  })

  const routed = () => {
    const address = view && addressOf(view)
    const elapsedMs = performance.now() - startedAt
    const timed = startedAt > 0 && elapsedMs < clickWindowMs
    if (!address || (!timed && address === ctx.$.current)) return
    startedAt = 0
    show(address, 'router', timed ? Math.max(1, Math.round(elapsedMs)) : 0)
  }

  const follow = () => {
    const address = view && addressOf(view)
    if (address && address !== ctx.$.current && !startedAt) routed()
  }

  const swapped = () => {
    if (startedAt) routed()
  }

  const markStart = event => {
    if (event.type === 'submit' || event.target.closest?.('a[href]')) startedAt = performance.now()
  }

  const leaving = () => ctx.update(current => { current.loading = true })

  const detach = () => {
    clearInterval(poll)
    swaps?.disconnect()
    if (!view) return
    view.removeEventListener('click', markStart, true)
    view.removeEventListener('submit', markStart, true)
    view.removeEventListener('popstate', follow)
    view.removeEventListener('pagehide', leaving)
    view.navigation?.removeEventListener('currententrychange', routed)
    view = null
  }

  const attach = () => {
    detach()
    view = frame.contentWindow
    view.addEventListener('click', markStart, true)
    view.addEventListener('submit', markStart, true)
    view.addEventListener('popstate', follow)
    view.addEventListener('pagehide', leaving)
    if (view.navigation) {
      view.navigation.addEventListener('currententrychange', routed)
      return
    }
    poll = setInterval(follow, pollMs)
    const page = view.document.querySelector('main[data-page]')
    swaps = new MutationObserver(swapped)
    if (page) swaps.observe(page, { childList: true })
  }

  const loaded = event => {
    if (event.target !== frame || !addressOf(frame.contentWindow)) return
    startedAt = 0
    attach()
    show(addressOf(view), 'load', loadMs(view))
  }

  document.addEventListener('load', loaded, true)
  if (frame.contentDocument?.readyState === 'complete') loaded({ target: frame })
  return () => {
    document.removeEventListener('load', loaded, true)
    detach()
  }
}
