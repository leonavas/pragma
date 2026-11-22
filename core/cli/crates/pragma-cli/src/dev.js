// WebSocket, not EventSource: an EventSource per tab uses up the 6 HTTP/1.1 connections per host.
let boot = null
let overlay = null

connect()

function connect() {
  const socket = new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/_pragma/reload`)
  socket.addEventListener('message', (event) => {
    const { type, data } = JSON.parse(event.data)
    if (type === 'hello') {
      if (boot && boot !== data) location.reload()
      boot = data
    } else if (type === 'reload') {
      location.reload()
    } else if (type === 'css') {
      swapCss()
    } else if (type === 'error') {
      showOverlay(JSON.parse(data))
    }
  })
  // the dev server restarted or is down: retry, and `hello` reloads if it is a new process
  socket.addEventListener('close', () => setTimeout(connect, 1000), { once: true })
}

function swapCss() {
  hideOverlay()
  for (const link of document.querySelectorAll('link[rel="stylesheet"]')) {
    const url = new URL(link.href, location.href)
    if (url.origin !== location.origin || !url.pathname.endsWith('.css')) continue
    const next = link.cloneNode()
    // In dev CSS names do not change (/assets/app.css, /assets/css/<route>.css): without ?t= the browser does not refetch.
    next.href = url.pathname + '?t=' + Date.now()
    // the old sheet is removed only after the new one loads, to avoid a flash of unstyled content
    next.addEventListener('load', () => link.remove(), { once: true })
    next.addEventListener('error', () => next.remove(), { once: true })
    link.after(next)
  }
}

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
