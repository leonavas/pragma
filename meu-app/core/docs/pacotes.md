# Pacotes npm

Use pacotes npm só quando a plataforma não resolve. Antes de instalar, veja se `Intl`, `fetch`,
`crypto.randomUUID`, `URLSearchParams` ou `structuredClone` já fazem o serviço.

## Instalar

```sh
npm i date-fns --ignore-scripts
bun add date-fns
```

- O pacote vai em `dependencies` do `package.json`. Import de pacote fora dele é erro.
- Qualquer gerenciador serve (npm, bun, pnpm). O pragma lê `package.json` e `node_modules/`.
- `node_modules/` fica fora do git.

## Importar

```js
import { format } from 'date-fns/format'
import { ptBR } from 'date-fns/locale/pt-BR'

export const state = () => ({ hoje: format(new Date(), 'PPP', { locale: ptBR }) })
```

- Import com aspas, nunca template literal.
- Prefira o subcaminho da função (`date-fns/format`). Sem bundler, importar o índice do pacote
  baixa todos os módulos dele, e o orçamento de JS estoura.
- O build copia cada módulo do pacote para `/assets/npm/` com hash e `modulepreload`. O peso
  conta no orçamento da página.
- `process.env.NODE_ENV` vira `"production"` no build e `"development"` no dev.

## O que funciona

Só pacote de JS puro em ES module (`import`/`export`, arquivos `.js` ou `.mjs`).

| Não funciona | Erro |
|---|---|
| CommonJS (`require`, `module.exports`) | `app/pacote` |
| TypeScript, JSX, `.vue`, `.svelte`, CSS importado | `app/pacote` |
| `node:*` ou módulo do Node no cliente | `app/pacote` |
| Framework de UI (React, Preact, Vue, Svelte, Solid, lit, jQuery, Alpine…) | `js/import` |

No `.server.js`, qualquer pacote do `package.json` funciona, inclusive CommonJS e `node:*`.

## Recomendados

| Para | Use |
|---|---|
| Formatar data, número, moeda, plural, tempo relativo | nativo: `Intl.DateTimeFormat`, `Intl.NumberFormat`, `Intl.PluralRules`, `Intl.RelativeTimeFormat` |
| Calcular com datas (somar, diferença, início do mês) | `date-fns` (subcaminho por função); fuso: `@date-fns/tz` |
| Validar formulário ou payload | `valibot` |
| Busca aproximada em lista | `fuse.js` |
| Gráfico | `chart.js` (desenha num `<canvas>` do `.html`) |
| Debounce, throttle, agrupar, ordenar | `es-toolkit` (subcaminho: `es-toolkit/function`, `es-toolkit/array`) |
| ID único | nativo: `crypto.randomUUID()` |
| HTTP | nativo: `fetch` |

## Não use

| Pacote | Motivo | Use |
|---|---|---|
| `moment`, `dayjs` | CommonJS ou pesado | `date-fns`, `Intl` |
| `lodash` | CommonJS | `es-toolkit` |
| `axios` | pesado; `fetch` resolve | `fetch` |
| `uuid`, `nanoid` | nativo resolve | `crypto.randomUUID()` |
| lib que monta HTML (`marked`, templates, componentes prontos) | HTML em JS é proibido | marcação no `.html` |
