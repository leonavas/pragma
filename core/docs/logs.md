# Logs

OpenTelemetry por padrão, no cliente e no servidor. Não configure nada.
Referência: `exemplo/routes/loja/[id].js` e `[id].server.js`.

## Exemplo canônico

```js
export const actions = {
  async salvar(ctx) {
    try {
      await ctx.call('salvar', { id: ctx.params.id })
      ctx.log.info('pedido salvo', { 'pedido.id': ctx.params.id })
    } catch (e) {
      ctx.log.error('falha ao salvar pedido', { 'pedido.id': ctx.params.id, 'exception.message': e.message })
      ctx.state.erro = 'Não foi possível salvar.'
    }
  },
}
```

Servidor: `load` e actions recebem `log` (`log.info(...)`). Em `utils/`:
`import { log } from '../core/log.js'`.

## Regras

- `log.debug|info|warn|error(mensagem, atributos)`.
- Mensagem curta e fixa; o que varia vai nos atributos.
- Atributos com nomes das semantic conventions do OTel quando existirem
  (`http.request.method`, `url.path`, `error.type`, `exception.message`); do domínio, com
  ponto: `pedido.id`, `produto.id`.
- `debug` para diagnóstico (desligado em produção), `info` para fato de negócio, `warn` para
  algo recuperável, `error` para falha.

## Automático (não repita)

- Erros não tratados, `unhandledrejection`, navegação e falha de fetch no cliente.
- Span por request no servidor; erro não tratado vira `ERROR` + 500.
- `traceparent` em todo fetch do runtime: cliente e servidor no mesmo trace.

## Não faça

- Não use `console.*`: o `pragma check` barra.
- Não logue senha, token, cookie, documento, e-mail, corpo de request ou dado pessoal.
- Não logue em laço nem a cada tecla.
