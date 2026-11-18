# Mercadinho no pragma

O app de `tests_1/SPEC.md` feito no pragma. É a referência canônica para agentes: cada recurso
do framework aparece aqui pelo menos uma vez.

## Como rodar

```sh
cd core/exemplo
pragma dev            # http://localhost:5180, live reload
pragma check --strict # erros + avisos de acessibilidade
pragma build && pragma serve   # dist/ como em produção (http://127.0.0.1:3000)
```

Testes de ponta a ponta (Playwright, Chromium e Firefox), fora do app:

```sh
cd core/exemplo/e2e && npm install && npx playwright install chromium firefox
PRAGMA=/caminho/do/pragma npm test     # sobe pragma serve e pragma dev em cópias do app
```

`e2e/mercadinho.test.mjs` cobre o app em `pragma serve`; `e2e/dev.test.mjs`, o `pragma dev`
(SSR, live reload de CSS sem recarregar). Variáveis: `PRAGMA`, `PLAYWRIGHT_DIR`, `BROWSERS`,
`PORT`, `DEV_PORT`.

## O que há em cada rota

| Rota | Arquivos | Recursos |
|---|---|---|
| layout | `routes/_layout.html` | nav com `aria-current` automático; badge do carrinho (`shared.carrinho.quantidade`) |
| `/` Painel | `routes/index.html`, `index.js` | `c-indicador` ×3 (aliases `valor`/`tom`, getters de tom no `state(ctx)`); `c-contador` ×3 com props diferentes (uma dentro de `c-abas`); `c-relogio` (`mount` com cleanup); `c-tema` (`core/theme.js`, sem piscar); `c-abas` (slots nomeados, ← →); `c-modal` |
| `/tarefas` | `routes/tarefas/index.*` | store persistido `tarefas` com getters e métodos; formulário com validação; lista com chave e edição inline (`autofocus` em bloco novo); `c-busca`; `c-filtros` com `?filtro=` na URL; `c-dropdown` + `c-alert` com `ctx.confirm` |
| `/loja` | `routes/loja/index.*` | `load` em `index.server.js` (fonte `routes/loja/_produtos.json`, cópia de `tests_1/shared/produtos.json`, 400 ms de atraso) com skeleton, erro com `?falhar=1` e filtro `?categoria=` no servidor; busca no cliente por getter; `_components/produto-card`, `comprar` (props via `data-attr-data-*`), `resumo-carrinho` (alert com `confirm`) |
| `/loja/:id` | `routes/loja/[id].*` | `ssr = true`; reusa `c-comprar`; action de servidor `curtir` via `ctx.call` com log OTel nos dois lados |
| 404 | `routes/_404.html` | |

Stores: `lib/state/carrinho.js` e `lib/state/tarefas.js` (persistidos). Utilitários puros:
`utils/dinheiro.js`, `utils/texto.js`. Classes custom em `lib/theme.css`: `card`, `btn*`,
`campo`, `nav-link`, `dialog`; o resto são utilitários.

`pragma build` (prod): JS da 1ª visita a `/` 10,3 kB gzip (soma por arquivo), CSS 4,7 kB, build
em ~20 ms. Medições de browser (FCP, perfis lentos) ficaram para a validação do autor.

## Desvios do `tests_1/SPEC.md`

- **Counter dentro do ProductCard.** No pragma o pai não lê o estado do filho. O seletor de
  quantidade com "Adicionar" é o componente `comprar` (estado `qtd`, limites por props), usado no
  card e no detalhe. O `contador` é reusado 3× no Painel.
- **Cache da loja.** Voltar à loja refaz o `GET /_data/loja` (dados são do servidor por
  navegação; cache de dados está fora da v0). O atraso de 400 ms fica no `load`.
- **Filtro de categoria** é feito no servidor (`load({ query })`); a busca por texto, no cliente.
- **"Tentar de novo"** é um link para `/loja` (sem `?falhar=1`).
- **Tabs** com 3 abas fixas (props `a`, `b`, `c` e slots de mesmo nome): props são texto, não listas.
- **Curtir** (`/loja/:id`) é extra: mostra action de servidor; o contador vive em memória no servidor.
- Estado inicial das tarefas vem do store (sem `localStorage`), como pede o SPEC.

## Bugs de integração corrigidos (framework)

- Dono léxico de slot (compilador marca `data-slot`, runtime avalia no escopo de quem escreveu;
  validação de actions e `state` também léxica).
- Scroll lock sem salto: `gutter` só quando havia barra.
- Modulepreload: `import()` dinâmico fora; `overlay.js` lazy e pré-carregado só onde há overlay.
- `state(ctx)` + `ctx.props` (props do `<c-x>` viram `data-*` na raiz de componente com JS);
  getters derivados em state e store (fora do `localStorage`).
- `data-with-x="state…"` em componente com JS vira erro; `data-model` aceita alias em componente.
- `aria-current` automático (build + router); tema sem piscar (`core/theme.js` + script inline).
- `data-attr-checked/selected` ligam a propriedade; bloco novo com `[autofocus]` recebe foco.
- `self.__pragmaDev` nunca era ligado em dev; agora o build de dev o injeta.
- `/loja/index.html` (HTML que o router busca) era tratado como SSR de `/loja/:id`: caminhos
  `.html` nunca são SSR (Rust e handler).
- Live reload de CSS não rebaixava `/assets/app.css` (mesmo nome): agora com `?t=`.
- `pragma serve/dev` deixavam o node órfão em SIGTERM.
- Avisos `a11y/*` no `pragma check` e `--strict`.

## Para a doc

- `diretivas.md`/`componentes.md`: dono léxico (conteúdo de slot é de quem escreveu; raiz do
  componente com JS é do componente); `data-slot` é gerado; `data-model` com alias em componente;
  proibido alias para `state` em componente com JS; `data-attr-checked/selected` ligam a
  propriedade; `[autofocus]` em bloco novo.
- `componentes.md`/`estado.md`: `state(ctx)`, `ctx.props` (props viram `data-*`; runtime com
  `data-attr-data-*`), getters para valores derivados (state e store), métodos no store; trocar
  "valor derivado atualizado pela action" por getter.
- `rotas.md`: `aria-current="page"` automático em links sem query; filtros marcam
  `aria-current="true"` por getter.
- `css.md`/`overlays.md`: scroll lock com `data-scroll-lock="gutter"`; tema com
  `core/theme.js` (`theme()`, `setTheme()`), script inline automático.
- `performance.md`: modulepreload só do estático + overlay onde há overlay; `import()` para
  código tardio.
- `cli.md`: `pragma check --strict`, lista de regras `a11y/*`; SIGTERM encerra o node; `.html`
  nunca é SSR.
- `dados.md`: exemplo de `load` lendo JSON local (`import … with { type: 'json' }`) e helper
  `_x.server.js` privado na pasta da rota.
- `estrutura.md`: arquivos `_nome.*` dentro de `routes/` são privados (helpers de servidor, dados).

## Pendente

- Medições de browser (FCP perfil lento, requisições, comparação com `tests_1/COMPARACAO.md`).
- Reexecutar a suíte e2e depois das últimas correções (`dev.js` e SIGTERM): a última execução
  completa em `pragma serve` passou 24/24 em Chromium e Firefox; o teste de live reload de CSS
  no Chromium falhava antes da correção do `dev.js` e não foi reexecutado.
