# Decisões do pragma

O que foi decidido e por quê. O **como** está em `SPEC.md`, que é o contrato da v0 e
vale sobre este arquivo. Mudar uma decisão **[firme]** exige decisão explícita do autor.

Status: **[firme]** · **[firme v0]** contrato da v0, revisável depois · **[provável]** ·
**[proposta]** · **[em aberto]** · **[substituída]** · **[descartada]**. Data de todos os itens: 2026-11-18.

## Resumo: o contrato do pragma

1. JavaScript puro (ES modules). Nunca TypeScript, nem JSDoc de tipos.
2. Estrutura de pastas fixa. Toda página é um `.html` em `routes/`; o caminho é a rota.
3. Nunca HTML em JS, nem no servidor. A marcação vive em `.html`.
4. Diretivas fechadas, chatas e determinísticas: uma forma de fazer cada coisa.
5. Componentes reaproveitados; cada página serve o mínimo (módulos por página + modulepreload).
6. A página abre rápido: HTML pré-gerado com skeleton, dados por loader async. SSR é opt-in.
7. Overlays sempre HTML + JS; voltar fecha o overlay; modal e alert travam o scroll.
8. CSS: subconjunto do Tailwind v4 em Rust; classe custom só para casos refinados.
9. Logs em OpenTelemetry por design; o app não usa `console.*`.
10. Servidor em JS no formato fetch handler; ferramenta local em Rust (`pragma`).
11. Documentação simples, enxuta e própria para LLMs.

Base empírica: `../tests_1/COMPARACAO.md` (SvelteKit SPA × astra × fable). Os nativos
entregaram cerca de 4× menos na 1ª visita (10,8 a 12,3 kB gzip contra 41,5 kB) e pintaram
em metade do tempo sem throttling (108 contra 212 ms). n = 1, um modelo por app.

## Linguagem

- **[firme]** Nunca TypeScript: nem `.ts`, nem build com TS, nem suporte opcional.
- **[firme]** Nem JSDoc de tipos (`@type`, `@param {…}`, `@typedef`, `@ts-*`, `tsc`).
- **[firme]** Sem comentários, em nada: app e framework (JS, HTML, CSS, Rust). O código é a
  documentação. No app, comentário e nome vago são erro de `pragma check` (`*/comentario`,
  `js/nome-vago`). *Motivo:* comentário envelhece e gasta contexto; nome bom não.

## HTML, diretivas e componentes

- **[firme]** Marcação só em `.html`. Nada de `innerHTML`, `createElement`, `h()` ou template
  literal com markup. *Motivo:* agentes dominam HTML e CSS, e é o mais fácil de revisar.
- **[firme]** Páginas pré-geradas no build; o JS liga comportamento a nós que já existem.
  Proibido re-renderizar o que veio pronto (sem hidratação que refaz o DOM).
- **[firme]** Diretivas fechadas, chatas e determinísticas, estilo Go (SPEC §3): caminhos
  `raiz.a.b` sem expressões; o que não está na lista é erro. *Motivo:* uma forma de fazer
  cada coisa reduz a variância dos agentes. Resolve "como o JS acha os nós" e a "sintaxe de
  componente".
- **[firme]** Reaproveitar componentes. `<c-nome>` é expandido no build (macro), com props
  literais e slots; o `.js` do componente só carrega nas páginas que o usam (SPEC §3).
- **[firme]** Dado do usuário e listas dinâmicas vêm de `<template>` (`data-if`, `data-for`
  com `data-key`); atualizações fazem diff por chave.
- **[firme]** Overlays (modal, alert, popup, dropdown) são sempre HTML + JS. Voltar (browser
  ou Android) fecha o overlay em vez de voltar a página. Modal e alert travam o scroll
  (SPEC §6).
- **[substituída]** "Sintaxe de componente" e "como o JS encontra os nós" (em aberto antes):
  resolvidas pelas diretivas.

## Estrutura e rotas

- **[firme]** A estrutura de pastas do app é fixa (SPEC §2).
- **[firme]** Toda página visível é um `.html` em `routes/`; o caminho do arquivo é a rota.
- **[firme]** `index.html` responde pela pasta; `x.html` e `x/index.html` juntos são erro de
  build; `[nome]` é path param; estático vence param; `_` no nome exclui da rota.
- **[firme]** Path params e query string no cliente e no servidor.
- **[firme v0]** Router no cliente: troca o `<main data-page>` pelo HTML pré-gerado da rota,
  com prefetch no hover, foco e viewport (SPEC §7). Substitui o "MPA × router" em aberto.

## Dados e SSR

- **[firme]** Direcional: a página abre rápido e baixa o que precisa com load async e
  loaders, mostrando skeleton (SPEC §5).
- **[firme]** Padrão sem SSR. Dados em `routes/<rota>.server.js` com `load(...)`; o cliente
  busca `/_data/<caminho>`, e o fetch já sai de um script inline no `<head>`, antes dos
  módulos.
- **[firme]** SSR é opt-in por página com `export const ssr = true`; o cliente liga as
  diretivas sem reconstruir o DOM.
- **[firme]** Nunca HTML em JS, nem no SSR: o servidor renderiza a partir dos mesmos `.html`,
  compilados em ops (`templates/<rota>.json`).
- **[firme v0]** Escrita pelo servidor via `actions` do `.server.js` e `ctx.call` →
  `POST /_action/<caminho>/<nome>`.
- **[substituída]** SSR via `HTMLRewriter` nos Workers e `lol_html` no `pragma serve`:
  trocado pelas ops compiladas, renderizadas pelo handler JS.
- **[substituída]** "SSR fica de fora (SPA + API)".

## CSS

- **[firme]** Reaproveitar CSS ao máximo.
- **[firme]** Subconjunto do Tailwind v4 reimplementado em Rust (crate `pragma-css`),
  gerando só as classes usadas, sem valores arbitrários. O objetivo é economizar classes;
  classe custom é permitida para casos refinados (SPEC §8). *Motivo:* o CSS para de crescer,
  os agentes já conhecem os nomes e não há decisões de nomenclatura.
- **[firme v0]** `base.css` (preflight + layers) e `lib/theme.css` (tokens e classes
  custom); tema escuro com `[data-theme="dark"]` e `dark:`; um CSS global com hash.
- **[substituída]** "Sem Tailwind, só o preflight" e "Tailwind v4 × CSS próprio".
- **[substituída]** CSS por rota: a v0 gera um CSS global (CSS por rota está fora da v0).

## Reatividade e estado

- **[provável]** v0: as actions mutam o estado e o runtime reaplica as diretivas. Não há
  signals expostos (SPEC §4). Estado global em stores (`lib/state/`), com `persist` opcional.
  Valor derivado é getter no `state(ctx)` ou no store, recalculado a cada render (não é
  copiado pela action nem persistido).
- Achado do `tests_1`: signals × store + `dispatch` quase não pesa (1,5 kB gzip de
  diferença; 3 a 10 ms sem throttling e 20 a 40 ms com CPU 4× em 500 tarefas, com vitórias
  dos dois lados). No fable, "escrever em signal dentro de effect lança erro" pegou um bug
  real: `blur` síncrono ao ocultar o campo focado disparava uma escrita dentro do effect.

## Performance

- **[firme]** O app tem de ser muito rápido. Cada página serve o mínimo: módulos por página
  e por componente, com `modulepreload` de todos (SPEC §9).
- **[firme v0]** Sem bundler: cada módulo minificado (oxc) com hash; assets `immutable`; HTML
  sem cache.
- **[firme v0]** Orçamentos em `pragma.json` (`initialJsKb`, `routeJsKb`, `cssKb`),
  checados no build e no check. Teto de referência: 7,2 a 8,2 kB gzip de JS total nos
  nativos do `tests_1`.
- **[substituída]** Chunk por rota com bundler e manifesto de chunks: a v0 usa módulos
  soltos e o manifesto `/_pragma/manifest.json`.

## Dependências npm

- **[firme]** Pacotes npm são permitidos. *Motivo:* é um projeto de dev web; proibir o
  ecossistema afasta pessoas e agentes. Substitui "sem pacotes npm".
- **[firme]** Instalação com o gerenciador que a pessoa quiser (npm, bun, pnpm); o pragma
  não tem gerenciador próprio, só lê `package.json` (`dependencies`) e `node_modules`.
- **[firme v0]** Cliente: só pacote de JS puro em ES module. Sem bundler: o build resolve o
  import (`oxc_resolver`, conditions `browser`/`import`/`module`) e trata cada módulo do pacote
  como os do app (minificado, hash, `/assets/npm/...`, `modulepreload`, conta no orçamento).
  CommonJS, TS, JSX, `.json`/CSS e `node:*` são erro `app/pacote`. `process.env.NODE_ENV` é
  substituído. *Motivo:* zero dependência nova de bundler e o mesmo modelo dos módulos do app.
- **[firme]** Guard-rails: import de pacote fora de `dependencies` é erro `js/import`;
  frameworks de UI (React, Vue, Svelte, lit, jQuery…) são recusados no cliente; os docs
  (`docs/pacotes.md`) trazem a lista recomendada e a de "não use".
- **[firme]** Servidor: pacote do `package.json` importado normalmente (inclusive CJS), sem
  resolver no build; Node acha `node_modules` subindo de `dist/server`; o build grava
  `server/package.json` com `"type": "module"`. Workers: `wrangler` empacota.
- **[descartada]** Pré-empacotar dependências com `rolldown` + import map: mais peso e
  complexidade; reavaliar se índices sem tree-shaking virarem problema.
- **[descartada]** CDN em produção (esm.sh, jsdelivr) e vendoring manual como padrão.

## Observabilidade (OTel)

- **[firme]** Logs em OpenTelemetry por design, no cliente e no servidor, sem configuração
  (SPEC §12): OTel Logs Data Model, semantic conventions, `traceparent` em todo fetch do
  runtime, span por request, erro não tratado vira `ERROR` + 500.
- **[firme]** Sem dados pessoais por padrão (sem corpo, cookies ou headers de auth).
- **[firme]** Cliente com logger próprio, sem o SDK oficial (pesado demais); envio em lote
  para `/_otel` na mesma origem.
- **[firme]** Exportação OTLP/HTTP (JSON) se `OTEL_EXPORTER_OTLP_ENDPOINT` existir, senão JSON
  OTel no stdout. Sem gRPC.
- **[firme]** Amostragem e nível mínimo configuráveis; `DEBUG` desligado em produção. O
  `pragma dev` mostra os logs legíveis, agrupados por trace.
- **[firme]** O app não usa `console.*`, e sim `log.*`; o `pragma check` barra.
- **[substituída]** Endpoint `/api/otel`: agora `/_otel`.

## Servidor e deploy

- **[provável]** Servidor em JS, formato `fetch(request, env, ctx)`. *Motivo:* Workers não
  é Node (isolates V8; Rust só como Wasm, com custo de tamanho, cold start e fronteira de
  strings). Rust fica no CLI.
- **[provável]** O `pragma dev` roda Node (≥ 22) para os loaders e faz proxy de `/_data`,
  `/_action`, `/_otel` e páginas SSR (SPEC §11, §15). Substitui o workerd no dev.
- **[provável]** Cloudflare Workers é o alvo preferido (`dist/server/worker.js` +
  `wrangler.jsonc`); Node via `dist/server/node.mjs`; hospedagem só estática quando não há
  `*.server.js`.
- **[provável]** Sem aprisionamento a um host: interfaces pequenas (`db`, `kv`, `files`)
  sobre o `env`. A v0 expõe `env` direto ao `load`.
- **[substituída]** Prefixo `/api/*` e `pragma serve` repassando para Node/Deno/Bun: agora
  `/_data`, `/_action`, `/_otel`, e o `serve` é o servidor do dev sem watch.

## Ferramenta (CLI Rust)

- **[firme]** App em Rust para rodar local, buildar e checar: `pragma new`, `dev`, `build`,
  `check`, `serve`, em binário único. *Motivo:* melhor ecossistema de JS/CSS (oxc,
  lightningcss), segurança de memória e binário único; Go e C++ descartados.
- **[firme]** `pragma check` valida tudo: JS, HTML, diretivas, CSS e orçamentos (SPEC §10).
  Confirmado pelo autor (2026-11-18).
- **[firme]** O `check` emite avisos de acessibilidade (não bloqueiam; `--strict` os torna
  erros): img sem alt, controle sem nome acessível, input sem label, dialog sem nome,
  `data-on-click` em elemento não interativo, níveis de heading pulados, mais de um h1,
  `tabindex` > 0, id duplicado, iframe sem title, `<a>` sem href.
- **[firme v0]** `hyper` + `tokio` sem framework; `oxc`; `lightningcss`; `pragma-css`.
- **[firme v0]** Parser de HTML próprio e estrito no compilador: toda tag não void fecha
  explicitamente; erro com linha e coluna. Substitui o em aberto `lol_html` × `html5ever`.
- **[firme]** A CLI fica em Rust; reescrever em Node foi descartado. *Motivo:* binário único
  e boot instantâneo (bom para agentes que chamam a CLI em loop), parser nativo (oxc), e
  Node não traz ganho de segurança: pacote npm roda com as mesmas permissões que um binário.
- **[firme]** Distribuição via npm com binários pré-compilados por plataforma
  (`optionalDependencies`, modelo esbuild/biome) + shim JS mínimo; `npx pragma` funciona sem
  saber que é Rust. Também release no GitHub para quem não usa Node.
- **[firme]** Ao publicar, o projeto será aberto (open source) e auditável: binários
  gerados só no CI a partir do repo público, `npm publish --provenance` + attestations do
  GitHub, checksums assinados (sigstore), nenhum `postinstall`, e a CLI sem acesso à rede
  nem a nada fora do projeto. *Ainda não feito; só na publicação.*
- **[provável]** Ferramenta de release: `cargo-dist`. Meta: igualar o esbuild do `tests_1`
  (build em cerca de 0,33 s, dev em cerca de 0,4 s).
- **[proposta]** Build WASM/WASI opcional (ex.: `wasmtime --dir=.`) como sandbox real e
  para playground no browser.
- **[substituída]** "`pragma check` valida só JavaScript".

## Documentação

- **[firme]** Documentação simples, enxuta e própria para LLMs: `core/docs/`, copiada para
  `<app>/core/docs/`, mais um `AGENTS.md` curto no app. Regras imperativas e um exemplo
  canônico por assunto.

---

## Em aberto

- [ ] Workers e rotas com param: com `not_found_handling` SPA, `/loja/123` recebe o
      `index.html`; falta resolver para `loja/[id].html` (no Node, o manifesto resolve).
- [ ] Interfaces `db`, `kv` e `files` sobre o `env`.
- [ ] Valores padrão dos orçamentos em `pragma.json`.
- [ ] Publicação: pipeline de release (cargo-dist), pacotes npm por plataforma,
      provenance/attestations e checksums assinados (ver "Ferramenta (CLI Rust)").
- [ ] Pacotes npm: sem tree-shaking, importar o índice de um pacote grande baixa todos os
      módulos; hoje só o orçamento segura. Avaliar aviso ou pré-empacotar.
- [ ] Bundler depois da v0 (`rolldown` ou próprio sobre o oxc) e se ele compensa.
- [ ] CSS por rota e CSS crítico inline.
- [ ] Brotli ou gzip; pré-comprimir no build.
- [ ] 103 Early Hints na Cloudflare.
- [ ] Medir LCP, INP e navegação em 4G lenta simulada, com o bench do `tests_1`, contra o
      SvelteKit com prerender ou SSR.
- [ ] Fora da v0 (SPEC §13): SSG com dados, layouts aninhados, `[...rest]`, Speculation
      Rules, i18n.
