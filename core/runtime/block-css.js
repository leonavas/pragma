// The preload runs in parallel with the page data. Only new block or self-call bodies that
// need these styles wait. Existing controls and all other blocks can mount immediately.
export default function applyBlockCss(K, schedule) {
  const el = document.querySelector('link[data-pragma-blocks]:not([data-pragma-pending])')
  if (!el) return
  const blocks = K.cssBlocks = new Set(el.dataset.pragmaBlocks.split(' '))
  const finish = () => {
    clearTimeout(timeout)
    if (K.cssBlocks === blocks) {
      K.cssBlocks = null
      if (K.cssMiss) {
        K.cssMiss = false
        schedule()
      }
    }
  }
  // A stalled network releases the waiting bodies after 1.5 s. A late response still applies.
  const timeout = setTimeout(finish, 1500)
  el.onload = () => {
    if (el.isConnected) document.querySelector('[data-pragma-interim-css]:not([data-pragma-pending])')?.remove()
    finish()
  }
  el.onerror = () => {
    if (!el.isConnected) return finish()
    // Retry with the complete route sheet. Keep the initial styles if both requests fail.
    el.onerror = finish
    el.onload = () => {
      if (el.isConnected) for (const part of document.querySelectorAll('[data-pragma-route-css]:not([data-pragma-pending])')) {
        if (part !== el) part.remove()
      }
      finish()
    }
    el.href = el.dataset.pragmaFullCss
  }
  if (el.dataset.pragmaCssStatus === 'load') el.onload()
  else if (el.dataset.pragmaCssStatus === 'error') el.onerror()
  else if (el.rel !== 'stylesheet') el.rel = 'stylesheet'
}
