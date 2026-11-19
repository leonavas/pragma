# Logs

OpenTelemetry no cliente e no servidor, sem configuração. `ctx.log` em actions e `mount`; `log`
no `load` e nas actions de servidor; `import { log } from '../core/log.js'` em `utils/`.

```js
export const actions = {
  async salvar(ctx) {
    try {
      await ctx.call('salvar', { id: ctx.params.id })
      ctx.log.info('pedido salvo', { 'pedido.id': ctx.params.id })
    } catch (erro) {
      ctx.log.error('falha ao salvar pedido', { 'pedido.id': ctx.params.id, 'exception.message': erro.message })
      ctx.$.erro = 'Não foi possível salvar.'
    }
  },
}
```

| Nível | Quando |
|---|---|
| `debug` | diagnóstico (desligado em produção) |
| `info` | fato de negócio |
| `warn` | algo recuperável |
| `error` | falha |

- Mensagem curta e fixa; o que varia vai nos atributos (`pedido.id`, `exception.message`,
  `http.request.method`).
- Automático, não repita: erro não tratado, navegação, falha de fetch, span por request, `traceparent`.
- Em `pragma dev`, todo `log.*` do cliente também aparece no console do navegador.
- Proibido: `console.*`, log em laço ou a cada tecla, senha, token, cookie, e-mail ou dado pessoal.
