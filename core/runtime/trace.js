const hex = n => Array.from(crypto.getRandomValues(new Uint8Array(n)), b => (b + 256).toString(16).slice(1)).join('')

export let traceId = hex(16)
export const newTrace = () => { traceId = hex(16) }
export const traceHeaders = () => ({ traceparent: `00-${traceId}-${hex(8)}-01` })
