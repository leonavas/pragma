# pragma v0 — contrato de implementação

Fonte única para quem implementa o pragma. Decisões e motivos: `DECISOES.md`.
Mudar este contrato exige avisar o coordenador (@claude-16).

## 0. Princípios

- JavaScript puro (ES modules). Nunca TypeScript, nunca JSDoc de tipos.
- Nunca HTML em JS: sem `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`,
  `createElement`, nem template literal com markup. A marcação vive em `.html`.
  Exceção: o runtime (`core/`) clona `<template>` e aplica diretivas.
- Página abre rápido: HTML pré-gerado com skeleton, dados buscados em paralelo.
  Cada página baixa só o JS dos módulos que usa.
- Determinístico e fechado: uma forma de fazer cada coisa; o que não está aqui é erro.
- Sem comentários, em nada (app e framework). O código é a documentação: nomes que se explicam.
  No app, `pragma check` barra comentário e nome vago.

## 1. Repositório do pragma

```
core/
  DECISOES.md  SPEC.md  README.md
  docs/              docs para LLMs (copiadas para <app>/core/docs/)
  runtime/           JS do framework (copiado para <app>/core/)
    server/          handler fetch, SSR, adaptador Node, logs do servidor
  starter/           esqueleto de app usado por `pragma new`
  cli/               workspace Rust
    crates/pragma-diag      tipo Diagnostic compartilhado
    crates/pragma-css       subconjunto Tailwind + base CSS
    crates/pragma-jslint    regras de JS (oxc)
    crates/pragma-compiler  build, análise, diretivas
    crates/pragma-cli       binário `pragma`: new, dev, build, check, serve
  exemplo/           app de exemplo (gerado com `pragma new`)
```

## 2. Estrutura do app (fixa)

```
<app>/
  AGENTS.md            regras para agentes (CLAUDE.md aponta para ele)
  pragma.json          config: { "name", "budgets": { "initialJsKb", "routeJsKb", "cssKb" } }
  routes/
    _layout.html       layout raiz; contém <main data-page><slot></slot></main>
    _404.html
    index.html         → /
    index.js           comportamento da página (opcional)
    index.server.js    dados da página (opcional)
    loja/
      index.html       → /loja
      [id].html        → /loja/:id
      [id].js  [id].server.js
      _components/<nome>/<nome>.html|.js|.css   componentes só desta rota
  lib/
    components/<nome>/<nome>.html|.js|.css      componentes do app
      modal/  alert/  dropdown/                 vêm do starter
    state/<nome>.js    estado compartilhado (store)
    theme.css          tokens e classes custom globais
  utils/               funções puras JS
  public/              arquivos copiados como estão
  core/                framework (gerado; não editar)
```

Regras de rota:
- Todo `.html` em `routes/` é rota, exceto nomes e pastas que começam com `_`.
- `index.html` responde pela pasta. `x.html` e `x/index.html` juntos = erro de build.
- `[nome]` (arquivo ou pasta) = path param. Estático vence param.
- Arquivo de página é fragmento: conteúdo que entra no `<slot>` do `_layout.html`.
  Um `<title>` no topo do fragmento vira o título da página.
- Componente usado na página: `<c-nome>` = `lib/components/<nome>/`; `<p-nome>` =
  `_components/<nome>/` da pasta da página ou de uma acima, até `routes/`. Não encontrado = erro.

## 3. Sintaxe do HTML (lista fechada)

`:` liga valor, `.` liga classe, `@` liga evento, `$` é estado ou prop do dono. O build traduz
para os `data-*` internos que o runtime lê (`:text` → `data-text`, `@click` → `data-on-click`,
`:href` → `data-attr-href`, `.x` → `data-class-x`, `$x` → `state.x` ou prop). Escrever `data-*`
de diretiva, `state.` ou `props.` no fonte é erro.

Caminhos: só `raiz.a.b` (sem expressões, chamadas, índices). Raízes: `$x` (estado ou prop do
dono), `data` (retorno do `load`), `shared` (stores), `params` `query` (strings), `page`
(`page.loading`, `page.error`), variável do `:for`.

| Sintaxe | Onde | Efeito |
|---|---|---|
| `:text="p"` | elemento | `textContent` (escapado) |
| `:<atributo>="p"` | elemento | atributo; `false`/`null`/`undefined` remove; `true` = vazio |
| `.<classe>="[!]p"` | elemento | liga/desliga a classe |
| `:show="[!]p"` | elemento | `hidden` quando falso |
| `:if="[!]p"` / `:else` | `<template>` | bloco condicional; `:else` logo após `:if` |
| `:for="x in p"` + `:key="x.campo"` | `<template>` | repete; `:key` obrigatório |
| `:empty` | `<template>` logo após `:for` | lista vazia |
| `:model="$p"` | input/select/textarea | mão dupla; só `$` ou `shared` |
| `@<evento>="acao"` | elemento | chama `actions.acao` do dono |
| `@open="id"` `@close` | botão | abre/fecha overlay (seção 6) |

Regras: `<template>` com `:if|:else|:for|:empty` tem exatamente 1 elemento raiz. Caminho
inexistente em dev = erro. `!` só em `:if`, `:show` e `.classe`. `style` e `on*` inline são
proibidos. `:checked` e `:selected` também ligam a propriedade.

**Dono léxico.** O dono de `@`, `:model` e `$` é quem escreveu o elemento: no `.html` de um
componente com módulo, o componente; na página, a página. Conteúdo de slot pertence a quem o
escreveu. O build marca cada raiz de conteúdo de slot que tenha diretivas com `data-slot="n"`
(n = quantos componentes acima está o dono); o runtime avalia esse elemento no escopo de fora
daquele componente. As diretivas da raiz de um componente com módulo pertencem ao componente.

### Componentes

`<c-x>` = `lib/components/x/x.html`; `<p-x>` = `_components/x/x.html` da pasta da página ou de uma
acima, até `routes/`. 1 elemento raiz. Expandido no build (macro).
- Prop fixa `titulo="X"`: literal no build. Prop dinâmica `:total="caminho"`: avaliada no escopo
  de quem usa e atualizada a cada render. Dentro: `$titulo`, `$total` (HTML) e `ctx.$` (JS).
- Prop é só leitura; prop ausente = vazio/falso; prop com nome de chave do estado = erro.
- Componente com módulo não recebe `$` de quem usa via `:x` (lá dentro `$` é o dele).
- `<slot></slot>` recebe os filhos; `<slot name="acoes">` recebe filhos com `slot="acoes"`.
- O build marca a raiz com `data-c="x"`. `x.js` só carrega nas páginas que usam o componente.

### DOM de blocos renderizados (cliente e SSR fazem igual)

O `<template>` fica no DOM. Cada instância é inserida como irmã logo depois dele, com
`data-of="<tid>"` (id que o build põe no template como `data-tid`) e, em `for`,
`data-key="<valor>"`. Atualizações fazem diff por chave; não recriam nós existentes.

### Formato compilado (para SSR)

O build gera `templates/<rota>.json`: lista de ops.
```
["html", "<literal>"]
["text", "path"]
["attr", "nome", "path"]                    dentro da tag de abertura
["class", "estaticas", [["nome","path",neg]]]
["show", "path", neg]
["if", tid, "path", neg, [ops], [opsElse]]
["for", tid, "var", "path", "keyPath", [ops], [opsEmpty]]
```
Instâncias saem com `data-of` e `data-key` como acima. Bloco recém-inserido no cliente que
tenha `[autofocus]` recebe o foco.

## 4. Módulos de página e componente (cliente)

```js
export const state = ctx => ({
  aberto: false,
  get total() { return ctx.shared.carrinho.itens.length },
})
export const actions = {
  alternar(ctx) { ctx.$.aberto = !ctx.$.aberto },
  async salvar(ctx) { await ctx.call('salvar', { id: ctx.params.id }) },
}
export function mount(ctx) {
  const intervalo = setInterval(() => ctx.update(estado => { estado.agora = Date.now() }), 1000)
  return () => clearInterval(intervalo)
}
```
`ctx`: `$ data shared params query page event el scope log open close confirm navigate call update`
(`$` = estado + props do dono).
- `state(ctx)` recebe o ctx do dono (sem `event`/`el`/`scope` de evento); `ctx.data` só existe
  depois do load. Valor derivado = getter no objeto de estado ou do store, recalculado a cada
  render.
- Depois de cada action (sync ou async) o runtime reaplica as diretivas.
  Fora de actions, mudar estado só via `ctx.update(fn)`.
- `mount` é só para efeitos externos (timer, listener global, observer). Retorna cleanup,
  chamado ao sair da página/remover o componente.
- `lib/state/carrinho.js`: `export default store({ itens: [] }, { persist: 'carrinho' })`
  (`store` vem de `core/state.js`). Usado no template como `shared.carrinho.*` e no JS
  como `ctx.shared.carrinho`. Getters do store são derivados (não vão para o `localStorage`);
  métodos do store concentram as mutações usadas por várias páginas.

## 5. Dados, loaders e SSR

`routes/loja/[id].server.js`:
```js
export async function load({ params, query, request, env, log }) { return { produto } }
export const actions = { async salvar({ input, params, env, log }) { return { ok: true } } }
export const ssr = false
```
- Padrão sem SSR: o HTML pré-gerado chega com skeleton (`<template :if="page.loading">`).
  O cliente busca `GET /_data/<caminho>?<query>` → `{ "data": ... }` ou `{ "error": { "message" } }`.
- O build injeta no `<head>` das páginas com loader um script inline mínimo que dispara o
  fetch antes dos módulos: `self.__pd = fetch('/_data' + location.pathname + location.search)`.
  O runtime usa `self.__pd` se existir.
- `ctx.call('nome', input)` → `POST /_action/<caminho>/<nome>` com JSON → retorno da action.
- `ssr = true`: o servidor roda `load`, renderiza o template compilado e embute os dados em
  `<script type="application/json" id="pragma-data">`. O cliente liga as diretivas sem
  reconstruir o DOM.
- Erro no `load` → `page.error` no cliente; 500 com log ERROR no servidor.

## 6. Overlays: modal, alert, dropdown, popover

Sempre HTML + JS. Primitiva em `core/overlay.js`; componentes em `lib/components/`.
- Modal e alert: `<dialog>` com `showModal()`. Alert usa `role="alertdialog"`.
- Dropdown e popover: atributo `popover` nativo.
- Abrir: `<button @open="sobre">`. Fechar: `@close` dentro do overlay, Esc ou clique
  no fundo (modal). Em JS: `ctx.open(id)`, `ctx.close(id)`, `await ctx.confirm(id)` → o
  `value` do botão `@close` clicado (`'cancel'` se fechou de outro jeito).
- Voltar (browser ou Android) fecha o overlay do topo, nunca navega: abrir faz `pushState`;
  `popstate` com overlay aberto fecha e para; fechar pela UI consome a entrada com `history.back()`.
- Modal e alert travam o scroll da página (`html[data-scroll-lock]` com `overflow:hidden`). Se a
  página tinha barra de rolagem, o valor é `gutter` e o CSS reserva o espaço dela
  (`scrollbar-gutter: stable`); sem barra, nada é reservado. Sem salto de layout nos dois casos.
  Vários abertos = pilha; destrava quando a pilha esvazia.
- `core/overlay.js` só é importado em página com `<dialog>` ou `[popover]` (o build faz o
  modulepreload dele só nessas páginas).
- Foco volta ao elemento que abriu.

Uso:
```html
<button @open="sobre">Sobre</button>
<c-modal id="sobre" titulo="Sobre o app">
  <p>Conteúdo.</p>
  <button slot="acoes" @close value="ok">Fechar</button>
</c-modal>
```

## 7. Router (cliente)

- Intercepta cliques em `<a>` da mesma origem (sem `target`, `download` ou `data-reload`).
- Prefetch no `pointerenter`/`focus` e quando o link entra na viewport (idle): baixa o HTML
  pré-gerado da rota e faz `modulepreload` dos módulos dela (manifesto).
- Navegar: busca HTML, troca o conteúdo de `<main data-page>` e o `<title>`, desmonta a
  página anterior (cleanups), importa os módulos e dispara o loader em paralelo.
  O layout não é recriado; suas diretivas são reaplicadas.
- `popstate`: o gerenciador de overlay intercepta primeiro.
- Scroll: topo em navegação nova; restaura em voltar/avançar. Foco vai para o `<main>`.
- Links `href="/caminho"` (sem query nem hash) para a rota atual recebem `aria-current="page"`:
  o build marca no HTML pré-gerado e o router mantém a cada navegação.
- `document.startViewTransition` quando disponível.
- Manifesto: `/_pragma/manifest.json` → `[{ "pattern": "/loja/:id", "html": "/loja/[id].html", "modules": [...], "loader": true, "ssr": false }]`.

## 8. CSS

- `core/base.css`: preflight (Tailwind v4, MIT) + `@layer reset, tokens, base, components, utilities`.
- `lib/theme.css`: tokens (`--color-*`, `--font-*`, `--radius-*`, `--shadow-*`) e classes custom
  em `@layer components`.
- Utilitários: subconjunto fechado com nomes do Tailwind v4, gerado pelo `pragma-css` só com as
  classes usadas. Sem valores arbitrários (`w-[37px]` = erro). Lista em `docs/classes.md`
  (gerada pelo próprio crate).
- Classe no HTML precisa ser utilitário suportado ou estar definida no CSS do projeto
  (`lib/theme.css`, `*.css` de componentes). Senão, erro no check.
- Classe custom: quando a combinação se repete em 3+ lugares ou precisa de algo fora do
  subconjunto. CSS de componente fica no `<nome>.css` e usa `[data-c="nome"]` como escopo.
- Saída v0: um CSS global minificado (lightningcss) com hash.

## 9. Build (`pragma build`)

```
dist/
  client/                 HTML por rota, assets com hash, /_pragma/manifest.json
  server/worker.js        entrada Cloudflare Workers (se houver *.server.js)
  server/node.mjs         adaptador Node (node:http) com estáticos
  server/wrangler.jsonc   assets dist/client, not_found_handling SPA, run_worker_first
  templates/<rota>.json   só para rotas ssr
```
- JS: ES modules sem bundler na v0. Cada módulo minificado (oxc) com hash no nome; imports
  reescritos. Cada página recebe `<link rel="modulepreload">` só do que usa na primeira pintura:
  o fecho dos imports estáticos (nunca os de `import()`), mais `core/overlay.js` se a página
  tem overlay. O manifesto usa a mesma lista.
- Build de dev põe `<script>self.__pragmaDev=true</script>` no `<head>` (erros de caminho e de
  action em dev).
- `index.html` não é cacheado; assets com hash são `immutable`.
- Orçamentos de `pragma.json` checados no fim do build.

## 10. Check (`pragma check`)

Erros com arquivo:linha:coluna, regra e mensagem.
- JS (oxc): HTML em JS (seção 0); `console.*`; TS ou JSDoc de tipos (`@type`, `@param {`,
  `@returns {`, `@typedef`, `@ts-`); import fora do projeto e de `core/`; comentário
  (`js/comentario`); nome vago (`js/nome-vago`).
- HTML: sintaxe desconhecida ou antiga; raiz de caminho inválida; `:for` sem `:key`;
  if/for fora de `<template>`; `:else`/`:empty` fora de lugar; bloco sem raiz única;
  action inexistente no módulo dono; componente não encontrado; `style`/`on*` inline;
  `<script>` em `routes/` e `lib/`.
- CSS: classe desconhecida; valor arbitrário.
- Orçamentos (faz build em diretório temporário).
- Avisos de acessibilidade (`a11y/*`, não bloqueiam; `pragma check --strict` os torna erro),
  sobre a página expandida, apontando o arquivo de origem: `img-alt`, `nome-acessivel` (button
  ou a sem texto, `:text`, `aria-label`/`labelledby` ou `title`), `label` (input, select e
  textarea; `type=hidden|submit|reset|button|image` isentos), `dialog-nome`,
  `clique-nao-interativo` (`@click` fora de elemento interativo sem `role` e
  `tabindex`), `heading` (nível pulado), `h1-unico` (ramos if/else contam separados),
  `tabindex` (> 0), `id-duplicado`, `iframe-title`, `a-sem-href`.

## 11. Servidor e dev

- Código de servidor é JS no formato `fetch(request, env, ctx)`. Cloudflare Workers é o alvo
  preferido; Node ≥ 22 via `dist/server/node.mjs`.
- `pragma dev` (Rust, hyper + tokio, porta 5180): build incremental em `.pragma/dev/`,
  watch, live reload por SSE em `/_pragma/reload` (CSS troca sem recarregar). Se houver
  `*.server.js`, sobe `node .pragma/dev/server/node.mjs` numa porta interna e faz proxy de
  `/_data`, `/_action`, `/_otel` e páginas ssr.
- `pragma serve`: o mesmo servidor sem watch, sobre `dist/`. Ctrl+C ou SIGTERM encerram também o
  node filho.
- Caminho que termina em `.html` nunca é página SSR (é o HTML pré-gerado que o router busca),
  no `pragma dev/serve` e no handler.
- `pragma new <dir>`: copia `starter/` + `runtime/` → `<dir>/core/` + `docs/` → `<dir>/core/docs/`
  (embutidos no binário).

## 12. Logs (OpenTelemetry)

- `core/log.js` (cliente) e `core/server/log.js`: `log.debug|info|warn|error(msg, attrs)`.
  Registro no OTel Logs Data Model; atributos com semantic conventions.
- Cliente: captura `error`, `unhandledrejection`, navegação e falha de fetch; envia em lote
  para `POST /_otel` (`keepalive`/`sendBeacon` no `visibilitychange`). Todo fetch do runtime
  manda `traceparent`.
- Servidor: span por request; `/_otel` repassa para OTLP/HTTP se `OTEL_EXPORTER_OTLP_ENDPOINT`
  existir, senão imprime JSON OTel no stdout.
- `pragma dev` mostra os logs legíveis no terminal.

## 13. Fora da v0

SSG com dados (`entries()`), CSS por rota, layouts aninhados, `[...rest]`, bundler,
Speculation Rules, i18n.

## 14. Contrato build ↔ runtime

### HTML gerado por página
```html
<!doctype html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>…</title>
  <script>try{document.documentElement.dataset.theme=…}catch{}</script>   só se o app usa core/theme.js
  <link rel="stylesheet" href="/assets/app-<hash>.css">
  <script>self.__pd=fetch('/_data'+location.pathname+location.search,{headers:{accept:'application/json'}})</script>
  <link rel="modulepreload" href="/assets/…">            um por módulo da página
  <script type="application/json" id="pragma-route">{…}</script>
  <script type="module" src="/assets/core/boot-<hash>.js"></script>
</head>
<body>…conteúdo do _layout.html com <main data-page>…página…</main>…</body>
</html>
```
- `__pd` só em páginas com loader. Tags extras de `<head>` podem vir de um bloco `<head>` no
  início do `_layout.html`.
- `#pragma-route`: `{ "pattern", "page": "<url do módulo da página ou null>",
  "components": { "<nome>": "<url>" }, "loader": bool, "ssr": bool }`. Só componentes com `.js`.
- Os módulos de `core/` (boot, router, …) também saem com hash; imports entre eles reescritos.

### Interfaces JS entre arquivos do runtime
- `core/boot.js`: entrada. Lê `#pragma-route`, inicia log e router, monta a página.
- `core/page.js` importa `core/overlay.js` sob demanda (`overlays()`) e o exporta em `overlay`
  para o router (`overlay?.handlePopState`, `overlay?.closeAll`).
- `core/overlay.js` (dono: overlays): `initOverlays({ log })`, `open(id)`, `close(id)`,
  `confirm(id) → Promise<string>`, `handlePopState(event) → boolean` (true = consumido; o
  router não navega).
- `core/log.js` (dono: servidor/logs): `log` com `debug|info|warn|error(msg, attrs)`,
  `initClientLog({ endpoint: '/_otel', service })`, `traceHeaders() → { traceparent }`.
- `core/state.js` (dono: runtime cliente): `store(initial, { persist })`.
- Dono de um módulo de componente: léxico (§3). O runtime guarda o dono no escopo de cada
  elemento; `data-slot="n"` volta n componentes. Sem dono de componente, o dono é a página. O layout não tem
  módulo na v0: suas diretivas usam `shared`, `params`, `query`.
- `/_pragma/reload` (SSE) e `/_pragma/dev.js` (cliente do live reload) são servidos pelo
  `pragma dev`, injetado só em dev.

### CSS que o runtime espera do `base.css`
`html[data-scroll-lock]{overflow:hidden}`, `html[data-scroll-lock=gutter]{scrollbar-gutter:stable}`, `[hidden]{display:none!important}`,
`dialog::backdrop` padrão e `.skeleton` (bloco pulsante com `prefers-reduced-motion`).

## 15. Contrato servidor ↔ build

```js
// core/server/handler.js
export function createHandler({ routes }) → { fetch(request, env, ctx) }
// routes: [{ pattern: '/loja/:id', server: <namespace do .server.js>, ssr: bool,
//            template: <ops compiladas> | null, shell: '<html da página pré-gerada>' | null }]
```
- Trata `/_data/*`, `/_action/*`, `/_otel` e páginas `ssr`. O resto: `env.ASSETS.fetch(request)`
  (Workers) ou o servidor de estáticos do adaptador Node.
- O build gera `dist/server/worker.js`: importa `./core/server/handler.js` e cada
  `.server.js` (copiados para `dist/server/` com seus imports relativos, preservando
  caminhos), embute templates e shells como objetos JS e faz `export default createHandler(...)`.
- O build gera `dist/server/node.mjs`:
  `import h from './worker.js'; import { serve } from './core/server/node.mjs'; serve(h, { root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000 })`.
- O adaptador Node serve estáticos de `client/` com MIME e cache certos, resolve rotas com
  param para o HTML delas via `client/_pragma/manifest.json` e responde `_404.html` com 404.

## 16. Paleta padrão

Cores que o `base.css` define e o `pragma-css` gera (`text-`, `bg-`, `border-`, `ring-`…):
`white`, `black`, `transparent`, `current` e `{neutral,primary,danger,success,warning}-{50,100,200,300,400,500,600,700,800,900,950}`.
O `lib/theme.css` pode redefinir valores e criar novas cores com `--color-<nome>` ou
`--color-<nome>-<tom>`; o gerador as reconhece. Escuro: `[data-theme="dark"]` no `<html>`
e variante `dark:`. Trocar o tema: `setTheme('dark'|'light')` e `theme()` de `core/theme.js`
(grava `localStorage['pragma:theme']`). Se algum módulo do app importa `core/theme.js`, o build
põe em toda página, antes do CSS, um script inline que restaura o tema (ou segue
`prefers-color-scheme`), sem piscar.
