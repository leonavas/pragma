const source = new EventSource('/_pragma/reload')
let boot = null
let overlay = null

source.addEventListener('hello', (event) => {
  if (boot && boot !== event.data) location.reload()
  boot = event.data
})

source.addEventListener('reload', () => location.reload())

source.addEventListener('css', (event) => {
  hideOverlay()
  for (const link of document.querySelectorAll('link[rel="stylesheet"]')) {
    const url = new URL(link.href, location.href)
    if (url.origin !== location.origin || !url.pathname.endsWith('.css')) continue
    const next = link.cloneNode()
    // In dev the CSS name does not change (/assets/app.css): without ?t= the browser does not refetch.
    next.href = (event.data && url.pathname.startsWith('/assets/') ? event.data : url.pathname) + '?t=' + Date.now()
    // the old sheet is removed only after the new one loads, to avoid a flash of unstyled content
    next.addEventListener('load', () => link.remove(), { once: true })
    next.addEventListener('error', () => next.remove(), { once: true })
    link.after(next)
  }
})

// a dropped connection also fires 'error', but as a plain Event without data
source.addEventListener('error', (event) => {
  if (event instanceof MessageEvent) showOverlay(JSON.parse(event.data))
})

function showOverlay(diagnostics) {
  hideOverlay()
  overlay = document.createElement('div')
  overlay.style.cssText = 'position:fixed;inset:0;z-index:2147483647;overflow:auto;padding:24px;background:rgba(20,20,24,.94);color:#f4f4f5;font:14px/1.5 ui-monospace,monospace'
  const title = document.createElement('strong')
  title.textContent = 'pragma: build error'
  title.style.cssText = 'display:block;margin-bottom:12px;color:#f87171;font-size:16px'
  const close = document.createElement('button')
  close.textContent = 'close'
  close.style.cssText = 'position:absolute;top:16px;right:16px;font:inherit'
  close.addEventListener('click', hideOverlay)
  const list = document.createElement('pre')
  list.style.cssText = 'margin:0;white-space:pre-wrap'
  list.textContent = diagnostics
    .map((d) => `${d.file}:${d.line}:${d.column} ${d.severity} [${d.rule}]\n  ${d.message}`)
    .join('\n\n')
  overlay.append(close, title, list)
  document.documentElement.append(overlay)
}

function hideOverlay() {
  overlay?.remove()
  overlay = null
}
