# Mercadinho: SvelteKit (Svelte 5 runes + SvelteKit 2)

Versões: `svelte@5.58.0`, `@sveltejs/kit@2.70.4`, `@sveltejs/adapter-static@3.0.11`,
`@sveltejs/vite-plugin-svelte@7.3.1`, `vite@8.3.8`, `typescript@6`. Node 26.

## 1. Como rodar

```sh
cd tests_1/sveltekit
npm install
npm run dev        # http://localhost:5171 (porta fixa em vite.config.ts)
npm run build      # gera build/ (estático: index.html de fallback + _app/)
npm run preview    # vite preview do build, http://localhost:5171
npm run check      # svelte-check (0 erros, 0 avisos)
```

Para servir o build como **arquivos estáticos puros com fallback para `index.html`**
(critério 10), sem adicionar dependência ao projeto:

```sh
npx sirv-cli build --single --port 5171
```

Teste de aceite automatizado (Playwright fica fora do projeto):

```sh
mkdir -p /tmp/pw && cd /tmp/pw && npm init -y && npm i playwright
cp <projeto>/e2e/aceite.mjs . && node aceite.mjs http://localhost:5171
```

## 2. Medidas (build de produção)

Gzip medido com `gzip -9 -c arquivo | wc -c`, somando os arquivos.

| Medida | Valor |
|---|---|
| JS total, gzip (19 `.js` em `build/`) | **45 593 B** (≈ 44,5 KiB). Sem compressão: 109 280 B |
| JS carregado na 1ª visita a `/`, gzip (13 arquivos, capturados no Playwright) | **37 289 B** (≈ 36,4 KiB) |
| CSS total, gzip (8 `.css`, preflight incluído) | **5 400 B**. Sem compressão: 14 396 B. Só o CSS da 1ª visita: 3 912 B |
| `index.html`, gzip | 828 B |

Composição do JS da 1ª visita (aproximada, pelo conteúdo dos chunks): runtime do Svelte ≈ 17,7 KB,
router/cliente do SvelteKit ≈ 10,3 KB e entry/manifesto ≈ 2,4 KB. Ou seja, cerca de **30 KB de
framework e ~7 KB de app**. O SvelteKit também pré-carrega o nó da página de erro (`nodes/1`) na
primeira visita.

**Linhas de código do app** (`src/` + configs, sem `preflight.css`, `node_modules`, saída de
build nem `e2e/`):

| Tipo | Linhas | Arquivos |
|---|---|---|
| `.svelte` | 1 196 (362 delas em blocos `<style>`) | 22 |
| `.ts` | 215 em `src/` (sem `app.d.ts`, que é do scaffold) + 7 de `vite.config.ts` | 10 |
| `.css` | 259 (`src/styles/app.css`) | 1 |
| `.html` | 22 (`src/app.html`) | 1 |
| `.js` | 16 (`svelte.config.js`) | 1 |

Total do app: ≈ 1 700 linhas, contando as em branco.

**Linhas de "runtime" próprio:** 0. Reatividade, componentes e router vêm do Svelte/SvelteKit.
O mais próximo de infraestrutura própria são `src/lib/state/persist.svelte.ts` (21 linhas,
sincroniza com o localStorage) e `src/lib/url.ts` (16 linhas, lê e grava filtros na URL).

## 3. Critérios de aceite

Todos verificados com Playwright (Chromium headless) pelo script `e2e/aceite.mjs`. A suíte inteira
passou contra **três servidores**:

- `npm run dev` (3 execuções seguidas);
- `npm run preview`;
- `build/` servido como estático por `sirv --single`.

| # | Status | Como foi verificado |
|---|---|---|
| 1 | ✅ | Navega pelos links do menu e pelo nome do produto até `/loja/cafe`, e volta pelo "← Voltar para a loja". Uma variável global (`window.__marcador`) sobrevive a toda a navegação, logo não houve reload. Também confere `<title>` por rota, `aria-current="page"` (ou `"true"` em `/loja/:id` para "Loja"), a 404 em `/nao-existe` com link para o início e "Produto não encontrado" em `/loja/xyz`. |
| 2 | ✅ | Deep link `/tarefas?filtro=concluidas` mostra só "Pagar contas". Os cliques em Pendentes e Todas mudam a URL. `goBack`/`goForward` restauram a lista e o botão `aria-pressed`, e o refresh mantém o filtro. Em `/loja?categoria=limpeza`, o filtro Hortifruti e o voltar também funcionam. Refresh em `/loja/cafe` e voltar a partir de `/` funcionam. |
| 3 | ✅ | "Cliques" chega a 3. "Pulos de 5" sobe até 50 (+ desabilita) e desce até 0 (− desabilita). O contador "Na aba" chega a 6, os outros não mudam e o valor sobrevive à troca de abas. Na loja, o Qtd da Alface (estoque 3) para em 3, o − fica desabilitado em 1 e o card da Banana continua em 1. No Detergente (estoque 0) aparecem o selo "Esgotado" e o botão "Adicionar" desabilitado. |
| 4 | ✅ | Adicionar 2 cafés atualiza o badge para 2 na hora; com 3 alfaces vai para 5. No CartSummary, o + fica desabilitado no estoque e o − baixa para 4. O total é R$ 74,80. No Painel aparecem "Itens no carrinho" = 4 (tom passa de `alerta` para `neutro`) e "Total do carrinho" = R$ 74,80. Remover e Esvaziar também foram testados. |
| 5 | ✅ | Validação: vazio, "ab", 81 caracteres e duplicata "  COMPRAR CAFÉ " (maiúsculas e espaços) dão erro; o erro some ao digitar. Enter adiciona, limpa o campo e devolve o foco. Também testados: concluir (StatCard "2 de 4"), editar com duplo clique (Enter salva, Esc cancela, duplicata rejeitada), busca "LAVAR", filtro Pendentes vazio mostrando "Nenhuma tarefa" e remover. Em "Limpar concluídas", Cancelar não muda nada e o foco volta ao botão; Confirmar remove as concluídas. |
| 6 | ✅ | Alterna o tema, adiciona uma tarefa, conclui outra e põe uma banana no carrinho. Depois de `reload()`, `data-theme`, o badge, o CartSummary, as tarefas, o checkbox e o `aria-pressed` do ThemeToggle continuam iguais. |
| 7 | ✅ | `setInterval`/`clearInterval` instrumentados por `addInitScript`, contando os intervalos de 1 s ativos e os ticks. Resultado: 0 ativos fora do Painel e 1 no Painel; o relógio muda após 1,6 s. Ao sair, ficam 0 ativos e os ticks param (1 → 1 em 2,5 s). Depois de 5 idas e voltas, continua 1 ativo no Painel. |
| 8 | ✅ | `.skeleton` visível em menos de 300 ms e `aria-busy="true"`; depois aparecem 9 produtos. A busca "zzz" mostra "Nenhum produto". Painel → Loja, detalhe → voltar: sem esqueleto e **1 único request** a `produtos.json` na sessão. `?falhar=1` mostra esqueleto e depois `role="alert"` com "Tentar de novo"; o retry mostra o esqueleto e falha de novo enquanto o parâmetro está na URL. |
| 9 | ✅ | "Sobre o app" abre pelo teclado (Enter). Esc, o botão Fechar e o clique no fundo fecham o diálogo, sempre com o foco de volta ao botão; clicar dentro não fecha. Tabs: → e ← trocam a aba e o foco, de forma circular; Home/End vão às pontas; o `tabindex` é móvel e Tab leva ao painel. |
| 10 | ✅ | `npm run build` gera `build/` estático. Servido por `sirv --single` (fallback para `index.html`), a suíte 1–9 passou inteira. `curl /loja/cafe` responde 200. |

Extra verificado: a ordem das camadas no browser é `reset,tokens,base,layout,components,utilities`,
e `[data-theme="dark"]` troca os tokens. Nenhum erro de console, exceto o log esperado do SvelteKit
para a rota inexistente (`Not found: /nao-existe`) e o 404 dessa mesma URL no dev.

## 4. Arquitetura: onde vive cada conceito

- **Estado global**: classes com campos `$state` em módulos `.svelte.ts`, exportadas como
  singletons.
  - `src/lib/state/todos.svelte.ts:15`, `carrinho.svelte.ts:10`, `catalogo.svelte.ts:15` (cache de
    produtos com `$state.raw`) e `theme.svelte.ts:8`.
  - Exporto o objeto, e não um `let` reatribuível, porque o Svelte não deixa exportar estado
    reatribuído.
- **Persistência**: `src/lib/state/persist.svelte.ts:15`, `persistir()` com
  `$effect.root` + `$effect`, que serializa o estado (leitura profunda = dependência profunda).
- **Reatividade derivada**:
  - `$derived` em campos de classe: `carrinho.svelte.ts:11-12`, `todos.svelte.ts:16-18`.
  - URL → lista filtrada em `src/routes/tarefas/+page.svelte:21-29`.
  - `page` de `$app/state` (não `$app/stores`).
- **Estado na URL**:
  - `src/lib/url.ts:5` (`lerParam`) e `:11` (`gravarParam`, com `goto(url, { keepFocus, noScroll })`,
    que cria entrada no histórico).
  - `FilterBar.svelte:12` deriva o valor de `page.url`.
- **Componente / props**:
  - `src/lib/components/Counter.svelte:13`: `$props()` com `value = $bindable(initial)`, para o
    `ProductCard` ler a quantidade com `bind:value`.
  - IDs de acessibilidade via `$props.id()`.
  - Props de callback: `TodoForm.svelte:2-7` (`validar`, `onadicionar`) e `Modal` (`onclose`).
- **Composição (snippets, sem `<slot>`)**:
  - `Tabs.svelte:4`: abas recebem `conteudo: Snippet`, renderizado em `:54`. As abas são
    declaradas com `{#snippet}` em `src/routes/+page.svelte`.
  - `Modal.svelte:9`: `acoes?: Snippet<[fechar]>`, renderizado em `:53`.
- **Rotas**: file-based em `src/routes/`.
  - `+layout.ts:2` (`ssr = false`, SPA); `svelte.config.js:12` (`adapter-static` com
    `fallback: 'index.html'`).
  - Rota dinâmica: `src/routes/loja/[id]/+page.svelte`.
  - 404: `src/routes/+error.svelte`.
  - Link ativo: `NavLink.svelte:8`.
  - `<title>` por `<svelte:head>` em cada página.
- **Efeito com limpeza**:
  - `src/lib/components/Clock.svelte:5-8`: `setInterval` + `return () => clearInterval`.
  - Outros efeitos: `ThemeToggle.svelte:5` (DOM externo + `localStorage`), `Modal.svelte:22`
    (`open` ↔ `showModal()`/`close()`) e `ProductGrid.svelte:12` (carga, com `untrack` para não
    depender do estado que a carga escreve).
- **Lista com chave**: `TodoList.svelte:49` (`{#each itens as todo (todo.id)}`),
  `ProductGrid.svelte:49` e `CartSummary.svelte:12`.
- **CSS**:
  - `src/styles/app.css`: `@import './preflight.css' layer(reset)`, mais os tokens nas camadas
    `tokens`/`base`/`layout`/`components`/`utilities`.
  - Cada `<style>` de componente é envolvido em `@layer components { … }`. Sem isso, o CSS escopado
    do Svelte ficaria fora de camada e venceria todas as camadas.
  - A ordem das camadas é declarada também em `src/app.html`. Motivo: no build, o minificador
    remove a declaração `@layer a, b, …;` do CSS, e no dev o CSS dos componentes entra antes do
    `app.css`.
- **Tema sem flash**: script inline em `src/app.html` aplica `data-theme` salvo antes do primeiro
  paint, em qualquer rota. O `ThemeToggle` (só no Painel) mantém `<html>` e `localStorage` em
  sincronia.

### Desvios da spec (justificados)

- **SvelteKit 2 fixado à mão.** Hoje, `npx sv create` (sv 1.5.0) gera SvelteKit **3**. Rebaixei para
  `@sveltejs/kit@2.70.4` + `adapter-static@3.0.11` e voltei à configuração clássica
  (`svelte.config.js`, alias `$lib`), como pede a spec.
- **Acréscimos de acessibilidade e UX, que não mudam o comportamento pedido:**
  - botão "Editar" em cada tarefa, como alternativa de teclado ao duplo clique;
  - `Home`/`End` nas Tabs;
  - na edição inline, o *blur* cancela;
  - `NavLink` usa `aria-current="true"` em subrotas (em `/loja/cafe`, "Loja" fica destacado) e
    `"page"` só na rota exata.
- **Carrinho:** além do "Esgotado", o "Adicionar" do `ProductCard` também desabilita quando o
  carrinho já tem todo o estoque daquele produto, e `carrinho.adicionar` limita ao estoque. O
  `CartSummary` também aparece em `/loja/:id`.
- **StatCards do Painel:** quando não estão em `alerta`, usam o tom `neutro`.
- **`?falhar=1` força o erro mesmo com cache.** "Tentar de novo" refaz a carga, que falha de novo
  enquanto o parâmetro estiver na URL. Sem `?falhar`, o cache é reaproveitado.

## 5. Pontos fracos e o que um agente erraria com mais facilidade

- **Sintaxe de Svelte 4 vazando.** Os dados de treino estão cheios de `export let`, `$:`,
  `on:click`, `<slot>`, `createEventDispatcher` e `$app/stores`/`$page`. Tudo isso ainda compila em
  parte (modo legado) ou dá erro só em runes mode. Forçar `runes: true` ajuda a pegar.
- **Deriva de versão.** `sv create` já entrega Kit 3, com config dentro do `vite.config.ts` e alias
  `#lib`. Um agente seguindo docs/exemplos de Kit 2 num projeto Kit 3, ou o contrário, quebra a
  configuração.
- **Regras de runes fora de componentes:**
  - `$state` só funciona em `.svelte`/`.svelte.ts`;
  - não dá para exportar um `let` com `$state` que é reatribuído;
  - `$effect` em módulo exige `$effect.root`;
  - um `$effect` que lê e escreve o mesmo estado entra em loop. Foi preciso `untrack` na carga dos
    produtos.
- **CSS escopado × `@layer`.** O CSS de componente sai *sem camada* e, portanto, vence todas as
  camadas. É fácil não perceber, porque parece funcionar até um utilitário não sobrescrever nada.
  Além disso, a ordem de carga do CSS por rota não é garantida, e por isso a declaração foi para o
  `app.html`.
- **SPA estática não é o default.** É preciso combinar `ssr = false` com `adapter-static`
  `fallback`. `vite preview` **não** testa o fallback estático de verdade (usa o servidor do Kit);
  é preciso servir `build/` com um servidor estático. Um detalhe que me pegou: o `sirv` sem `--dev`
  guarda a lista de arquivos em cache, então depois de um rebuild ele serve 404 até ser reiniciado.
- **Assincronia do `<dialog>`.** O evento `close` dispara depois do `close()`, e foco/estado tratado
  ali pode correr contra a próxima ação. Os testes precisaram esperar o foco voltar.
- **Muita coisa implícita.** Reatividade pelo compilador, rotas pelo sistema de arquivos e CSS
  separado por rota pelo bundler: o agente não "vê" o runtime e tende a supor comportamentos
  (ex.: `bind:this` num array comum não é reativo, defaults de `$bindable` valem só na montagem).
- **Tamanho.** ~30 KB gzip de JS da 1ª visita são framework (runtime do Svelte + router do Kit),
  contra ~7 KB de código do app.
