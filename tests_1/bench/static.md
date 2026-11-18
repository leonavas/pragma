# Mercadinho: medidas estáticas e de ferramenta

Medido em 2026-11-18 por @bench-static. Dados brutos em `static.json`. Builds, dev e `npm ci` rodaram em cópias em `/tmp/pragma-bench/<app>`; as pastas originais não foram tocadas. Todas as medições de tempo rodaram sob `flock /tmp/pragma-bench.lock`.

## Resumo

| Medida | sveltekit | astra | fable |
|---|---:|---:|---:|
| Tempo do agente | 21 min 45 s | 15 min 38 s | 22 min 47 s |
| Chamadas de ferramenta do agente | 37 | 56 | 33 |
| Código do app: arquivos / linhas / linhas de código | 32 / 1 692 / 1 510 | 10 / 830 / 828 | 27 / 1 388 / 1 195 |
| Runtime próprio: linhas de código | 0 | 109 | 323 |
| Tooling próprio: linhas de código | 0 | 22 | 107 |
| Bytes de fonte (app + runtime) | 44 893 | 49 451 | 55 741 |
| JS total: bruto / gzip / brotli (kB) | 109,3 / 45,6 / 40,8 | 23,7 / 8,2 / 7,4 | 18,8 / 7,2 / 6,5 |
| CSS total: bruto / gzip / brotli (kB) | 14,4 / 5,4 / 4,4 | 12,5 / 3,7 / 3,3 | 10,6 / 3,3 / 2,8 |
| Arquivos JS + CSS no build | 19 + 8 | 1 + 1 | 1 + 1 |
| 1ª visita a `/`: requisições / gzip / brotli (kB) | 17 / 41,5 / 37,1 | 3 / 12,3 / 10,9 | 3 / 10,8 / 9,6 |
| Dependências diretas (dev + prod) | 7 + 0 | 1 + 0 | 1 + 0 |
| Pacotes em node_modules / tamanho | 58 / 75M | 2 / 11M | 2 / 11M |
| Build de produção (mediana de 5) | 6,01 s | 0,34 s | 0,33 s |
| Dev: cold start até o 1º 200 (mediana de 5) | 2,60 s | 0,38 s | 0,40 s |
| `npm ci` com cache quente (1×) | 3,81 s | 0,96 s | 1,15 s |
| Checks do projeto | svelte-check ✅ (0 erros, 0 avisos) | nenhum check próprio | test ✅ 22/22, check ✅ |
| `console.*` no código do app | 0 | 0 | 0 |

## 1. Tempo de desenvolvimento pelo agente

O tempo vem de `TEMPOS.md`. As chamadas foram contadas em `sheltie agent read <handle> --full --last 5000`, filtrando pelo id do run de `TEMPOS.md`. Os runs anteriores de astra-signals e fable-signals ficaram de fora; as mensagens de cada run se isolam pelo campo `run`.

| | sveltekit | astra | fable |
|---|---:|---:|---:|
| Agente / modelo | @app-sveltekit / claude opus | @astra-signals / gpt-6-astra | @fable-signals / claude-fable-5-2 |
| Duração | 21 min 45 s | 15 min 38 s | 22 min 47 s |
| Chamadas de ferramenta (total) | 37 | 56 | 33 |
|   shell (bash) | 33 | 51 | 25 |
|   edição de arquivo (edit) | 1 | 0 | 3 |
|   ver imagem (Read / view_image) | 3 | 5 | 5 |
| Mensagens de texto do agente | 8 | 11 | 6 |

Atenção: os três agentes escreveram a maior parte dos arquivos por heredoc dentro do shell. Uma chamada de bash pode criar vários arquivos e rodar o build e o teste na mesma vez, então o número de chamadas não mede o volume de trabalho.

## 2. Código

"Código" conta as linhas não vazias que não são só comentário. Comentários reconhecidos: `//`, `/* */` e `<!-- -->`. O `<script>` e o `<style>` dentro de `.svelte` e `.html` são tratados pela linguagem de cada um.

### Código do app

| Tipo | sveltekit (arq / linhas / código) | astra | fable |
|---|---:|---:|---:|
| .js | – | 8 / 658 / 656 | 25 / 834 / 724 |
| .ts | 8 / 215 / 165 | – | – |
| .svelte | 22 / 1 196 / 1 078 | – | – |
| .css | 1 / 259 / 247 | 1 / 156 / 156 | 1 / 534 / 452 |
| .html | 1 / 22 / 20 | 1 / 16 / 16 | 1 / 20 / 19 |
| **total** | **32 / 1 692 / 1 510** | **10 / 830 / 828** | **27 / 1 388 / 1 195** |
| bytes de fonte | 44 893 | 45 635 | 41 766 |

### Runtime próprio

| Tipo | sveltekit (arq / linhas / código) | astra | fable |
|---|---:|---:|---:|
| .js | – | 4 / 111 / 109 | 4 / 448 / 323 |
| **total** | **0 / 0 / 0** | **4 / 111 / 109** | **4 / 448 / 323** |
| bytes de fonte | 0 | 3 816 | 13 975 |

### Tooling (build/dev/preview/check)

| Tipo | sveltekit (arq / linhas / código) | astra | fable |
|---|---:|---:|---:|
| .js | – | 1 / 22 / 22 | 3 / 126 / 107 |
| **total** | **0 / 0 / 0** | **1 / 22 / 22** | **3 / 126 / 107** |
| bytes de fonte | 0 | 1 001 | 5 249 |

O astra tem metade das linhas do sveltekit e quase os mesmos bytes de fonte. Suas linhas são longas e quase sem linhas em branco ou comentários. Para comparar o volume de código, use os bytes junto com as linhas.

### O que entrou em cada grupo

**sveltekit**

- app: `src/app.html` (22/20), `src/lib/components/AppHeader.svelte` (48/46), `src/lib/components/CartBadge.svelte` (33/31), `src/lib/components/CartSummary.svelte` (87/85), `src/lib/components/Clock.svelte` (33/28), `src/lib/components/Counter.svelte` (67/62), `src/lib/components/FilterBar.svelte` (38/32), `src/lib/components/Modal.svelte` (86/72), `src/lib/components/NavLink.svelte` (36/31), `src/lib/components/ProductCard.svelte` (83/78), `src/lib/components/ProductGrid.svelte` (84/77), `src/lib/components/SearchInput.svelte` (11/9), `src/lib/components/StatCard.svelte` (40/37), `src/lib/components/Tabs.svelte` (80/72), `src/lib/components/ThemeToggle.svelte` (13/10), `src/lib/components/TodoForm.svelte` (56/51), `src/lib/components/TodoList.svelte` (131/119), `src/lib/format.ts` (3/2), `src/lib/state/carrinho.svelte.ts` (47/36), `src/lib/state/catalogo.svelte.ts` (52/44), `src/lib/state/persist.svelte.ts` (21/15), `src/lib/state/theme.svelte.ts` (15/9), `src/lib/state/todos.svelte.ts` (59/46), `src/lib/url.ts` (16/12), `src/routes/+error.svelte` (17/14), `src/routes/+layout.svelte` (13/11), `src/routes/+layout.ts` (2/1), `src/routes/+page.svelte` (89/81), `src/routes/loja/+page.svelte` (31/25), `src/routes/loja/[id]/+page.svelte` (45/39), `src/routes/tarefas/+page.svelte` (75/68), `src/styles/app.css` (259/247)
- runtime: nenhum
- tooling: nenhum
- excluídos: `src/styles/preflight.css`, `src/styles/preflight.LICENSE`, `src/app.d.ts (boilerplate de tipos do template)`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `.npmrc`, `.vscode/`, `e2e/aceite.mjs`, `e2e/medir.mjs`, `static/*`, `build/`, `.svelte-kit/`

**astra**

- app: `index.html` (16/16), `src/app.js` (48/48), `src/components/common.js` (225/224), `src/components/shop.js` (99/99), `src/components/todos.js` (89/89), `src/pages/dashboard.js` (36/36), `src/pages/shop.js` (36/35), `src/pages/tasks.js` (41/41), `src/state.js` (84/84), `src/styles.css` (156/156)
- runtime: `src/lib/component.js` (27/26), `src/lib/list.js` (33/32), `src/lib/router.js` (25/25), `src/lib/store.js` (26/26)
- tooling: `scripts/build.mjs` (22/22)
- excluídos: `src/preflight.css`, `public/*`, `scripts/acceptance.mjs`, `scripts/deep-links.mjs`, `scripts/measure.mjs`, `evidence/`, `dist/`

**fable**

- app: `index.html` (20/19), `src/components/app-header.js` (35/31), `src/components/cart-summary.js` (34/32), `src/components/clock.js` (10/9), `src/components/counter.js` (21/19), `src/components/filter-bar.js` (18/14), `src/components/modal.js` (38/34), `src/components/product-card.js` (26/24), `src/components/product-grid.js` (37/34), `src/components/search-input.js` (12/10), `src/components/stat-card.js` (13/11), `src/components/tabs.js` (50/43), `src/components/theme-toggle.js` (11/10), `src/components/todo-form.js` (40/39), `src/components/todo-list.js` (65/57), `src/format.js` (15/10), `src/main.js` (19/16), `src/pages/loja.js` (61/57), `src/pages/nao-encontrada.js` (8/7), `src/pages/painel.js` (71/70), `src/pages/tarefas.js` (73/71), `src/state/cart.js` (43/29), `src/state/persisted.js` (21/17), `src/state/products.js` (38/26), `src/state/theme.js` (20/14), `src/state/todos.js` (55/40), `src/styles/app.css` (534/452)
- runtime: `src/lib/component.js` (34/22), `src/lib/dom.js` (106/75), `src/lib/reactive.js` (239/176), `src/lib/router.js` (69/50)
- tooling: `scripts/build.js` (57/48), `scripts/check.js` (41/34), `scripts/preview.js` (28/25)
- excluídos: `src/styles/preflight.css`, `src/styles/preflight.LICENSE`, `public/*`, `test/reactive.test.js`, `dist/`

No sveltekit, `svelte.config.js`, `vite.config.ts` e `tsconfig.json` são configuração e ficaram fora. O runtime e o roteador vêm do framework, em `node_modules`. No astra e no fable, o `index.html` conta como código do app, no papel equivalente ao `src/app.html` do SvelteKit.

## 3. Bundle de produção

O build foi feito do zero na cópia com `npm run build`. A compressão usou `gzip -9` e `brotli -q 11` (CLIs), arquivo por arquivo. Os tamanhos estão em bytes.

### sveltekit

| Arquivo | bruto | gzip | brotli | 1ª visita |
|---|---:|---:|---:|:-:|
| `/_app/immutable/chunks/DkSoK_Ua.js` | 46 645 | 17 698 | 16 087 | ✓ |
| `/_app/immutable/chunks/BcZrWdMs.js` | 26 755 | 10 265 | 9 308 | ✓ |
| `/_app/immutable/nodes/5.BdxFJ5qb.js` | 5 359 | 2 410 | 2 105 |  |
| `/_app/immutable/entry/app.Dlf73n1Z.js` | 5 254 | 2 356 | 2 068 | ✓ |
| `/_app/immutable/nodes/2.Ck4VXCmp.js` | 5 148 | 2 370 | 2 116 | ✓ |
| `/_app/immutable/chunks/C-J4ZThy.js` | 4 414 | 2 009 | 1 789 |  |
| `/_app/immutable/nodes/3.CtRAaVVl.js` | 2 967 | 1 478 | 1 293 |  |
| `/_app/immutable/chunks/bH1TpTz6.js` | 2 884 | 1 448 | 1 279 | ✓ |
| `/_app/immutable/nodes/0.Bsy5n1dw.js` | 1 867 | 931 | 812 | ✓ |
| `/_app/immutable/nodes/4.CL370rx_.js` | 1 825 | 1 004 | 876 |  |
| `/_app/immutable/chunks/Bjy-W4x2.js` | 1 365 | 666 | 499 |  |
| `/_app/immutable/chunks/DDFFAbdB.js` | 1 287 | 739 | 635 |  |
| `/_app/immutable/chunks/D6rj5LTu.js` | 1 177 | 670 | 588 | ✓ |
| `/_app/immutable/chunks/WbpY05qh.js` | 963 | 485 | 431 | ✓ |
| `/_app/immutable/nodes/1.CAGJG9n8.js` | 712 | 495 | 425 |  |
| `/_app/immutable/chunks/BNnSyLbg.js` | 294 | 179 | 139 | ✓ |
| `/_app/immutable/chunks/DBQEbcSe.js` | 239 | 202 | 175 | ✓ |
| `/_app/immutable/chunks/xihTtKlq.js` | 65 | 91 | 70 | ✓ |
| `/_app/immutable/entry/start.Cv45uePc.js` | 60 | 95 | 65 | ✓ |
| `/_app/immutable/assets/0.0SexZWbg.css` | 9 084 | 2 896 | 2 485 | ✓ |
| `/_app/immutable/assets/ProductCard.DnD9qt8M.css` | 1 635 | 586 | 461 |  |
| `/_app/immutable/assets/5.ClQuSy6Q.css` | 1 006 | 428 | 335 |  |
| `/_app/immutable/assets/todos.IG_BZB4n.css` | 972 | 438 | 359 | ✓ |
| `/_app/immutable/assets/2.Cdy0xb6S.css` | 618 | 334 | 259 | ✓ |
| `/_app/immutable/assets/3.DhcxsyOZ.css` | 504 | 280 | 210 |  |
| `/_app/immutable/assets/format.BpuQej35.css` | 341 | 244 | 182 | ✓ |
| `/_app/immutable/assets/SearchInput.VS6OUTNU.css` | 236 | 194 | 127 |  |
| `/index.html` | 1 875 | 825 | 659 | ✓ |
| **total JS (19 arq.)** | **109 280** | **45 591** | **40 760** | |
| **total CSS (8 arq.)** | **14 396** | **5 400** | **4 418** | |
| **1ª visita a / (17 arq.: 12 JS, 4 CSS, HTML)** | **104 241** | **41 527** | **37 082** | |

Nº total de arquivos na saída do build: 31 (inclui `produtos.json` e similares).

### astra

| Arquivo | bruto | gzip | brotli | 1ª visita |
|---|---:|---:|---:|:-:|
| `/assets/app.js` | 23 666 | 8 188 | 7 377 | ✓ |
| `/assets/app.css` | 12 497 | 3 747 | 3 256 | ✓ |
| `/index.html` | 508 | 326 | 230 | ✓ |
| **total JS (1 arq.)** | **23 666** | **8 188** | **7 377** | |
| **total CSS (1 arq.)** | **12 497** | **3 747** | **3 256** | |
| **1ª visita a / (3 arq.: 1 JS, 1 CSS, HTML)** | **36 671** | **12 261** | **10 863** | |

Nº total de arquivos na saída do build: 5 (inclui `produtos.json` e similares).

### fable

| Arquivo | bruto | gzip | brotli | 1ª visita |
|---|---:|---:|---:|:-:|
| `/assets/app-4ANYNGKT.js` | 18 811 | 7 152 | 6 486 | ✓ |
| `/assets/app-6VBXMAM3.css` | 10 587 | 3 265 | 2 802 | ✓ |
| `/index.html` | 681 | 432 | 294 | ✓ |
| **total JS (1 arq.)** | **18 811** | **7 152** | **6 486** | |
| **total CSS (1 arq.)** | **10 587** | **3 265** | **2 802** | |
| **1ª visita a / (3 arq.: 1 JS, 1 CSS, HTML)** | **30 079** | **10 849** | **9 582** | |

Nº total de arquivos na saída do build: 4 (inclui `produtos.json` e similares).

A 1ª visita foi calculada por análise estática. Ela soma o `index.html`, os scripts, os stylesheets, os `modulepreload` e os `import()` do HTML, e segue os imports estáticos recursivamente. No SvelteKit também entram os nós que o router carrega para `/`: o layout `0` e a página `2`, com as dependências JS e CSS listadas em `__vite__mapDeps`. Ficam de fora o chunk da página de erro, as outras rotas e o `produtos.json`, que só é buscado em `/loja`. O preflight.css está embutido no CSS dos três builds.

## 4. Dependências

| | sveltekit | astra | fable |
|---|---|---|---|
| devDependencies (versão instalada) | @sveltejs/adapter-static 3.0.11<br>@sveltejs/kit 2.70.4<br>@sveltejs/vite-plugin-svelte 7.3.1<br>svelte 5.58.0<br>svelte-check 4.7.8<br>typescript 6.0.3<br>vite 8.3.8 | esbuild 0.27.7 | esbuild 0.25.12 |
| dependencies | nenhuma | nenhuma | nenhuma |
| Pacotes instalados em node_modules | 58 | 2 | 2 |
| `du -sh node_modules` | 75M (73 756 433 B) | 11M (11 267 300 B) | 11M (10 496 999 B) |

A contagem de pacotes vem de `node_modules/.package-lock.json`, depois de um `npm ci` limpo, e inclui o binário de plataforma do esbuild e do lightningcss. O astra e o fable usam versões diferentes do esbuild: 0.27 e 0.25.

## 5. Ferramenta

| | sveltekit | astra | fable |
|---|---:|---:|---:|
| Build: mediana | 6,01 s | 0,34 s | 0,33 s |
| Build: 5 execuções (ms) | 6007, 5830, 6226, 5586, 6121 | 339, 365, 309, 369, 289 | 320, 332, 307, 369, 383 |
| Dev cold start: mediana | 2,60 s | 0,38 s | 0,40 s |
| Dev cold start: 5 execuções (ms) | 2730, 2464, 2597, 2431, 2865 | 379, 417, 406, 384, 341 | 366, 402, 406, 351, 440 |
| `npm ci` (cache quente, 1×) | 3,81 s | 0,96 s | 1,15 s |

Método:
- **Build:** `npm run build`. Houve 1 rodada de aquecimento e depois 5 rodadas intercaladas entre os apps, cada uma sob a trava. O tempo inclui o overhead do `npm run`, de ~0,1–0,2 s. No SvelteKit são `vite build` e o adapter-static. No astra e no fable é um script com esbuild.
- **Dev cold start:** conta do spawn de `npm run dev` até o primeiro HTTP 200 em `/`, com curl a cada ~10 ms. Antes de cada execução, as saídas e os caches de dev foram apagados: `.svelte-kit` e `node_modules/.vite` no sveltekit, `dist/` no astra e `.dev/` no fable. As portas foram 5181, 5182 e 5183. No sveltekit a porta veio de `-- --port 5181`, e no fable de `PORT=5183`. O astra tem a porta 5172 fixa em `scripts/build.mjs`, então **só na cópia** troquei para 5182. O 200 do Vite chega antes da otimização de dependências; o tempo até a página ficar interativa no dev é maior que esse número.
- **`npm ci`:** `npm ci --no-audit --no-fund`. Um `npm ci` de aquecimento rodou antes. No sveltekit, o tempo inclui o `prepare` (`svelte-kit sync`). O npm 12 bloqueou o postinstall do esbuild (`allowScripts`), mas o build funciona porque o binário vem no pacote `@esbuild/linux-x64`.

## 6. Qualidade estática

| App | Check | Resultado |
|---|---|---|
| sveltekit | npm run check (svelte-kit sync && svelte-check) | ✅ passa — 0 erros, 0 avisos (5,71 s) |
| astra | (nenhum check/teste estático no package.json; scripts/acceptance.mjs é e2e com Playwright, fora do escopo) | — |
| astra | node --check em src/ e scripts/ (16 arquivos, check neutro aplicado por mim) | ✅ passa |
| fable | npm test (node --test) | ✅ passa — 22/22 testes (0,47 s) |
| fable | npm run check (lint do dialeto, scripts/check.js) | ✅ passa — nenhum problema (0,24 s) |
| fable | node --check em src/ e scripts/ (32 arquivos, check neutro aplicado por mim) | ✅ passa |

O astra não tem typecheck, lint nem teste unitário. Seu `scripts/acceptance.mjs` é um teste e2e com Playwright e um servidor rodando, então não é estático e ficou fora daqui. O `node --check` (só sintaxe) foi aplicado por mim no astra e no fable para ter uma base comum mínima.

| `console.*` | sveltekit | astra | fable |
|---|---:|---:|---:|
| código do app | 0 | 0 | 0 |
| runtime próprio | 0 | 0 | 0 |
| tooling | 0 | 0 | 3 |

Os 3 `console.*` do fable estão nos scripts de tooling: 2 no `check.js` e 1 no `preview.js`.

## Limitações

- Chamadas de ferramenta: a contagem vem do resumo que o Sheltie gera de cada run, com um comando de shell por entrada e as edições e leituras de imagem como entradas próprias. Os harnesses diferem: o claude usou `Read` e `edit`, e o gpt-6-astra usou `view_image` e escreveu tudo pelo shell. Com isso, o número de chamadas só é comparável de forma aproximada.
- Tempos: cada medição rodou sob a trava, mas a máquina não estava ociosa, porque outros agentes e os servidores 5171–5173 estavam ativos. Os tempos têm ruído de alguns décimos de segundo; a ordem de grandeza é confiável.
- 1ª visita: a soma vem de análise estática, não de captura de rede. O `data-sveltekit-preload-data="hover"` pode baixar chunks de outras rotas quando o usuário passa o mouse nos links, e isso não está contado.
- Linhas: o removedor de comentários é léxico e simples (não interpreta regex literal). Conferi por amostragem em `reactive.js`, `Modal.svelte` e `index.html`.
