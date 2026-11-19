# Logs

OpenTelemetry on client and server, zero config. Use `ctx.log` in actions and `mount`; `log`
in `load` and server actions; `import { log } from '../core/log.js'` in `utils/`.

```js
export const actions = {
  async save(ctx) {
    try {
      await ctx.call('save', { id: ctx.params.id })
      ctx.log.info('order saved', { 'order.id': ctx.params.id })
    } catch (failure) {
      ctx.log.error('order save failed', { 'order.id': ctx.params.id, 'exception.message': failure.message })
      ctx.$.error = 'Could not save.'
    }
  },
}
```

| Level | When |
|---|---|
| `debug` | diagnostics (off in production) |
| `info` | business event |
| `warn` | recoverable problem |
| `error` | failure |

- Message: short and fixed. Variable parts go in attributes (`order.id`, `exception.message`, `http.request.method`).
- Already automatic, do not repeat: unhandled errors, navigation, fetch failures, one span per request, `traceparent`.
- In `pragma dev`, every client `log.*` also prints to the browser console.
- Forbidden: `console.*`; logging inside loops or on every keystroke; passwords, tokens, cookies, emails or any personal data.
