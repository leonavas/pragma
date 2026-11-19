let stack = []
let log, opener
let backs = 0
let pushes = 0

const get = id => typeof id == 'string' ? document.getElementById(id) : id
const entry = el => stack.find(e => e.el == el)
const hide = el => el.popover ? el.hidePopover() : el.close()
// Reserve the scrollbar gutter only if there was a scrollbar; otherwise it would shift the layout.
function lock() {
  const h = document.documentElement, on = stack.some(e => !e.el.popover)
  if (on == h.hasAttribute('data-scroll-lock')) return
  if (on) h.setAttribute('data-scroll-lock', innerWidth > h.clientWidth ? 'gutter' : '')
  else h.removeAttribute('data-scroll-lock')
}

// Each open overlay has a history entry. history.back() is async;
// a pushState before its popstate would be undone, so it is deferred.
const push = () => backs ? pushes++ : history.pushState(history.state, '')
const back = () => {
  if (pushes) pushes--
  else { backs++; history.back() }
}

function add(el, o) {
  if (entry(el)) return
  stack.push({ el, o })
  push()
  lock()
}

function done(el) {
  const e = entry(el)
  if (!e) return
  stack = stack.filter(x => x != e)
  lock()
  const a = document.activeElement
  if (!a || a == document.body || el.contains(a)) e.o?.focus()
  e.r?.(e.v || 'cancel')
  e.s || back()
}

function place(el, o) {
  const r = o.getBoundingClientRect(), s = el.style, h = el.offsetHeight
  s.margin = 0
  s.inset = 'auto'
  s.top = (r.bottom + h + 4 > innerHeight && r.top > h + 4 ? r.top - h - 4 : r.bottom + 4) + 'px'
  s.left = Math.max(4, Math.min(r.left, document.documentElement.clientWidth - el.offsetWidth - 4)) + 'px'
}

function onClick(ev) {
  const t = ev.target, b = t.closest('[data-open],[data-close]')
  if (t.localName == 'dialog' && t.getAttribute('role') != 'alertdialog') {
    const r = t.getBoundingClientRect(), x = ev.clientX, y = ev.clientY
    if (x < r.left || x > r.right || y < r.top || y > r.bottom) close(t)
  }
  if (!b) return
  const id = b.dataset.open
  if (id == null) return close(b.closest('dialog,[popover]'), b.value)
  const el = get(id)
  if (el?.popover) {
    ev.preventDefault()
    // As the invoker, the button does not trigger its own popover's light dismiss.
    b.popoverTargetElement = el
    if (el.matches(':popover-open')) return close(el)
  }
  open(id, b)
}

export function initOverlays(o = {}) {
  log = o.log
  document.addEventListener('click', onClick)
  document.addEventListener('close', e => done(e.target), true)
  document.addEventListener('beforetoggle', e => {
    const el = e.target
    if (!el.popover) return
    if (e.newState == 'open') add(el, opener || document.activeElement)
    else done(el)
    opener = null
  }, true)
}

export function open(id, o = document.activeElement) {
  const el = get(id)
  if (!el) return log?.error('overlay not found', { 'pragma.overlay.id': id })
  if (el.popover) {
    if (el.matches(':popover-open')) return
    opener = o
    el.showPopover()
    if (o && o != document.body) place(el, o)
  } else if (!el.open) {
    // showModal closes popovers; focus must return to their trigger.
    o = entry(o?.closest('[popover]'))?.o || o
    el.returnValue = ''
    el.showModal()
    add(el, o)
  }
}

export function close(id, v) {
  const el = get(id), e = entry(el)
  if (!e) return
  e.v = v
  hide(el)
}

export function confirm(id) {
  return new Promise(r => {
    open(id)
    const e = entry(get(id))
    e ? e.r = r : r('cancel')
  })
}

export function handlePopState() {
  stack = stack.filter(e => e.el.isConnected)
  lock()
  if (backs) {
    if (!--backs) for (; pushes; pushes--) history.pushState(history.state, '')
    return true
  }
  const e = stack.at(-1)
  if (!e) return false
  e.s = 1
  hide(e.el)
  return true
}

// For the router: closes everything without touching history. Returns how many overlay entries
// remain on top of history (the router may use replaceState instead of pushState).
export function closeAll() {
  const n = stack.length - pushes
  const all = stack
  stack = []
  pushes = 0
  lock()
  for (const e of all) {
    e.r?.('cancel')
    hide(e.el)
  }
  return n
}
