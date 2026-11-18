## Resumo

Medianas de 10 runs (ms, menor é melhor). Detalhes, p90 e metodologia abaixo.

| Medida | A sveltekit | A astra | A fable | B sveltekit | B astra | B fable |
|---|---:|---:|---:|---:|---:|---:|
| `/` FCP = LCP (carga fria) | 212 | 108 | 108 | 1118 | 760 | 646 |
| `/` bytes transferidos / requisições | 49,0 KB / 18 | 36,2 KB / 3 | 30,0 KB / 3 | idem | idem | idem |
| TTI prático (`/`, clique no + funciona) | 123 | 69 | 79 | 1035 | 744 | 711 |
| SPA Painel → Tarefas | 31 | 14 | 14 | 282 | 61 | 61 |
| SPA Tarefas → Loja, produtos visíveis (−400 ms) | 35 | 20 | 19 | 458 | 88 | 226 |
| Counter + / Adicionar → CartBadge | 13 / 16 | 13 / 15 | 13 / 15 | 25 / 36 | 20 / 30 | 17 / 22 |
| 500 tarefas: SPA até a lista | 135 | 86 | 76 | 728 | 411 | 371 |
| 500 tarefas: tecla na busca / filtro | 31 / 40 | 27 / 34 | 24 / 29 | 161 / 204 | 129 / 161 | 110 / 132 |
| Heap após 30 ciclos (Δ 10→30) | 3,73 MB (+213 KB) | 2,52 MB (+116 KB) | 2,58 MB (+73 KB) | — | — | — |
| Lighthouse mobile, score (FCP) | 100 (1235) | 100 (1171) | 100 (751) | | | |

**Leitura curta:**
- **Carga:** astra e fable pintam cerca de 2× mais cedo no perfil A e 30–42% mais cedo no
  B. O sveltekit roda com `ssr = false` (modo SPA): o HTML chega vazio e o app depende de
  18 módulos encadeados. Com RTT de 150 ms, essa cascata pesa mais que os bytes.
- **Navegação SPA:** no perfil A o sveltekit leva 2 frames (~31 ms) e os outros 1 frame
  (~15 ms). No B, a 1ª visita a cada rota do sveltekit baixa o chunk da rota com cache
  frio (~250 ms a mais). Já em rotas visitadas, como o `history.back`, a diferença cai
  para 10–20 ms.
- **Interações pequenas:** no A, os três ficam iguais, presos ao frame de 16,7 ms. No B
  (CPU 4×) a ordem é fable < astra < sveltekit, com diferenças de 3 a 14 ms.
- **500 tarefas:** fable é o mais rápido em tudo, exceto no Backspace no B, em que o astra
  ganha por pouco. O sveltekit fica de 30% a 100% mais lento que os nativos.
- **Memória:** nenhum vazamento de DOM ou de listeners nos três (contagens estáveis do
  ciclo 1 ao 30). O heap cresce um pouco e cada vez mais devagar (ver anomalias).
- **Lighthouse:** score 100 nos três, então não diferencia os apps. Só as métricas brutas
  diferem.

## Anomalias e limitações

1. **Compressão diferente entre servidores (não é do framework).** O `vite preview` do
   sveltekit serve com gzip; o servidor do esbuild (astra) e o `preview.js` (fable)
   servem **sem compressão**. Mesmo assim, astra e fable transferem menos (36 e 30 KB
   contra 49 KB). Com gzip, transfeririam cerca de 1/3 disso, e no perfil B a carga
   ficaria ainda mais rápida. Não alterei os servidores (pedido explícito).
2. **DOMContentLoaded/`load` do sveltekit não são comparáveis.** Com `ssr = false`, o
   SvelteKit inicia o app por `import()` dinâmico num `<script>` inline, que não segura o
   DCL nem o `load`. Por isso ele "ganha" nessas duas métricas mas perde no FCP e no TTI.
   Para comparar, use FCP, LCP e TTI prático.
3. **LCP de `/loja` no astra** é o parágrafo "Carregando produtos…" (os cards renderizados
   depois são menores e não substituem o candidato). O LCP do astra na loja, portanto, não
   mede "produtos visíveis". Para isso, use a linha "Tarefas → Loja (produtos visíveis)".
4. **Loja: o astra busca em paralelo com o atraso.** O astra faz
   `Promise.all([fetch, espera 400 ms])`; sveltekit e fable esperam 400 ms e *depois*
   fazem o fetch. No perfil A isso some; no B (RTT 150 ms) dá ~140 ms de vantagem ao
   astra em "produtos visíveis". É uma diferença de implementação, não da abordagem.
5. **Quantização por frame.** As latências medidas até o "próximo frame" saem em
   múltiplos de ~16,7 ms (60 Hz) no perfil A. Diferenças menores que um frame lá não são
   significativas. Os tempos até o DOM (`t_dom`, em `perf.json`) têm resolução fina: por
   exemplo, Painel→Tarefas no A dá ~23 ms (sveltekit), ~10 ms (astra) e ~6 ms (fable).
6. **TBT = 0 em todos.** A única long task relevante (avaliação do bundle e 1º render)
   termina antes do FCP, porque os três renderizam no cliente, e o TBT só conta depois do
   FCP. Por isso também reporto a soma das long tasks: no B, em `/`, ela é 344 ms
   (sveltekit), 184 ms (astra) e 120 ms (fable).
7. **Heap cresce um pouco nos 3** (de 1 para 30 ciclos: +645 KB no sveltekit, +294 KB no
   astra, +257 KB no fable). O ritmo cai pela metade ou mais entre os ciclos 1–10 e 10–30,
   e nós e listeners ficam constantes, o que indica aquecimento (JIT, inline caches,
   caches do router), não vazamento clássico. Não fiz snapshot de heap para provar. Os
   "Nodes" do CDP no sveltekit (486 → 670) incluem os `<template>` que o Svelte guarda num
   2º documento inerte (`Documents = 2`). Os elementos no documento real vão de 115 para
   123 por causa dos `<link>` de CSS das rotas injetados no `<head>` na 1ª visita.
8. **Memória só no perfil A.** Throttling de CPU ou rede não muda o que fica retido, e
   rodar no B custaria ~10 min a mais de trava.
9. **Hover do SvelteKit.** `data-sveltekit-preload-data="hover"`: o clique do Playwright
   passa o mouse sobre o link alguns ms antes do `pointerdown`, e o preload começa ali.
   Isso favorece o sveltekit, mas o efeito é desprezível perto dos números medidos.
10. **Hosts:** o sveltekit escuta só em `[::1]` (acessado como `localhost`); astra e fable
    em `127.0.0.1`. Há outro `vite dev`, de outro projeto, em `[::1]:5173`. Por isso o
    fable precisa ser acessado como `127.0.0.1:5173`, nunca como `localhost:5173`.
11. **Perfil B usa viewport desktop** (1280×720, igual ao A), para que só o throttling
    mude. O Lighthouse usa o próprio preset mobile (412×823, throttling **simulado**),
    por isso o FCP dele difere do perfil B medido.
12. **Ambiente:** uma única máquina, Chromium headless, sem isolamento de CPU. O p90 fica
    perto da mediana em quase tudo, então o ruído é baixo. As medições nunca rodaram em
    paralelo com as de build (trava `flock`), mas outros processos da máquina não foram
    controlados.
13. **Diferenças de UI que entram na medida:** o filtro do fable usa links (`<a>` +
    `pushState`); os outros usam `<button>`. O astra mostra mais texto por página (taglines,
    rodapé). Os seletores são por papel e texto, então medem a mesma ação do usuário.
