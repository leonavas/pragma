# Logs

Pragma configures OpenTelemetry on client and server automatically.

- In page, component, and store scripts, import `log` from `'pragma'`.
- In `load` and server actions, use the `log` argument.
- In `utils/`, import `log` from `'../.pragma/log.js'`.

```js
import { call, params, log } from 'pragma'

let error = ''

async function save() {
  try {
    await call('save', { id: params.id })
    log.info('order saved', { 'order.id': params.id })
  } catch (failure) {
    log.error('order save failed', { 'order.id': params.id, 'exception.message': failure.message })
    error = 'Could not save.'
  }
}
```

| Level | Use when |
|---|---|
| `debug` | Diagnostics (shown in `pragma dev`; off in production unless `PRAGMA_LOG_LEVEL=debug`). |
| `info` | Business event. |
| `warn` | Recoverable problem. |
| `error` | Failure. |

## Rules

- Keep messages short and fixed. Put dynamic values in attributes (`order.id`, `exception.message`, `http.request.method`).
- Do not duplicate automatic logs: unhandled errors, navigation, fetch failures, request spans, and `traceparent`.
- In `pragma dev`, client `log.*` also prints to the browser console.
- Client logs send to `POST /_otel`. On static hosts returning 404 or 405, the client stops sending.
- Forbidden: `console.*`, logging inside loops or on every keystroke, and logging passwords, tokens, cookies, emails, or personal data.
