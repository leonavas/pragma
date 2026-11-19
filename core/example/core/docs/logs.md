# Logs

OpenTelemetry on client and server, zero config. In page, component and store scripts use
`log` from `pragma`; in `load` and server actions the `log` argument; in `utils/`
`import { log } from '../core/log.js'`.

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

| Level | When |
|---|---|
| `debug` | diagnostics (shown in `pragma dev`; off in production unless `PRAGMA_LOG_LEVEL=debug`) |
| `info` | business event |
| `warn` | recoverable problem |
| `error` | failure |

- Message: short and fixed. Variable parts go in attributes (`order.id`, `exception.message`, `http.request.method`).
- Already automatic, do not repeat: unhandled errors, navigation, fetch failures, one span per request, `traceparent`.
- In `pragma dev`, every client `log.*` also prints to the browser console.
- Client logs go to `POST /_otel`; on a static host without it (404/405) the client stops sending.
- Forbidden: `console.*`; logging inside loops or on every keystroke; passwords, tokens, cookies, emails or any personal data.
