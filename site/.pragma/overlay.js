import { K, reveal } from './dom.js'
import { afterMotion } from './motion.js'

// Loaded by page.js on the first open()/close()/confirm(), or when a route with a <dialog> or
// [popover] enters. Its arrival runs one whole render, which enables the build-disabled
// data-open/data-close controls.

let stack = []
let opener
let backs = 0
let pushes = 0
let anchors = 0
const LEAVE = Symbol(), ANCHOR = Symbol()
const anchored = CSS.supports('position-try-fallbacks', 'flip-block')

const get = id => typeof id == 'string' ? document.getElementById(id) : id
const entry = el => stack.find(e => e.el == el)

function hide(el) {
  if (el.popover) return el.hidePopover()
  if (el[LEAVE]) return
  const e = done(el)
  const leave = () => {
    if (el[LEAVE] != leave) return
    el[LEAVE] = null
    el.removeAttribute('data-leaving')
    el.inert = false
    el.close()
    refocus(el, e?.o)
  }
  el[LEAVE] = leave
  el.setAttribute('data-leaving', '')
  el.inert = true
  afterMotion(el, leave)
}

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
  e.r?.(e.v || 'cancel')
  e.s || back()
  return e
}

function refocus(el, o) {
  const a = document.activeElement
  if (!a || a == document.body || el.contains(a)) o?.focus()
}

// data-placement="bottom-start" (default) | "bottom-end" | "top-start" | "top-end": side of the
// trigger, then the edge both share. Flips to the other side or edge when there is no room.
const GAP = 4
const AREA = { bottom: 'block-end', top: 'block-start', start: 'span-inline-end', end: 'span-inline-start' }
function place(el, o) {
  const [side, edge] = (el.dataset.placement || 'bottom-start').split('-'), s = el.style
  s.inset = 'auto'
  if (anchored) {
    const name = (el[ANCHOR] ??= '--pragma-anchor-' + anchors++)
    o.style.anchorName = name
    s.positionAnchor = name
    s.positionArea = AREA[side] + ' ' + AREA[edge]
    s.positionTryFallbacks = 'flip-block, flip-inline, flip-block flip-inline'
    s.margin = side == 'top' ? `0 0 ${GAP}px` : `${GAP}px 0 0`
    return
  }
  s.margin = 0
  s.top = s.left = 0
  const r = o.getBoundingClientRect(), h = el.offsetHeight, w = el.offsetWidth, vw = document.documentElement.clientWidth
  const below = r.bottom + h + GAP <= innerHeight, above = r.top >= h + GAP
  const up = side == 'top' ? above || !below : above && !below
  const left = edge == 'end'
    ? r.right - w >= GAP || r.left + w > vw - GAP ? r.right - w : r.left
    : r.left + w <= vw - GAP || r.right - w < GAP ? r.left : r.right - w
  s.top = (up ? r.top - h - GAP : r.bottom + GAP) + 'px'
  s.left = Math.max(GAP, Math.min(left, vw - w - GAP)) + 'px'
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

export function initOverlays() {
  document.addEventListener('click', onClick)
  document.addEventListener('close', e => e.target.open || refocus(e.target, done(e.target)?.o), true)
  document.addEventListener('cancel', e => {
    const el = e.target
    if (el.localName != 'dialog' || !e.cancelable || !entry(el)) return
    e.preventDefault()
    close(el)
  }, true)
  document.addEventListener('beforetoggle', e => {
    const el = e.target
    if (!el.popover) return
    if (e.newState == 'open') reveal(el), add(el, opener || document.activeElement)
    else refocus(el, done(el)?.o)
    opener = null
  }, true)
}

export function open(id, o = document.activeElement) {
  const el = get(id)
  if (!el) return K.log.error('overlay not found', { 'pragma.overlay.id': id })
  reveal(el)
  if (el.popover) {
    if (el.matches(':popover-open')) return
    opener = o
    el.showPopover()
    if (o && o != document.body) place(el, o)
  } else if (!el.open || el[LEAVE]) {
    // showModal closes popovers; focus must return to their trigger.
    o = entry(o?.closest('[popover]'))?.o || o
    el[LEAVE]?.()
    el.returnValue = ''
    el.showModal()
    // `autofocus` on the dialog: focus the dialog itself (WAI-ARIA, text-first content).
    if (el.autofocus) {
      if (!el.hasAttribute('tabindex')) el.tabIndex = -1
      el.focus()
    }
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
    e.el.popover ? e.el.hidePopover() : e.el.close()
  }
  return n
}
