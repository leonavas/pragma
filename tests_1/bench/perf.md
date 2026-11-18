# Desempenho no browser: sveltekit × astra × fable

Gerado por `bench/perf/` (Playwright + Chromium) em 2026-11-18T20:18:34.850Z. Dados brutos e
medianas em [`perf.json`](perf.json). Para reproduzir: `cd bench/perf && npm i &&
node bench.mjs load tti nav interact stress memory && node lighthouse.mjs 3 && node report.mjs`.

**Leitura das tabelas:** app nas colunas; tempos em ms como `mediana (p90)` de 10 runs;
contagens e bytes são a mediana (são determinísticos). Menor é melhor, exceto o score do
Lighthouse.

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

## Ambiente e versões

| Item | Valor |
|---|---|
| Chromium | 159.0.8154.6 (Chromium do Playwright, `channel: 'chromium'`, headless novo) |
| Playwright | 1.65.1 |
| Lighthouse | | Lighthouse (mediana de 3; valores individuais do score) | sveltekit | astra | fable |
|---|---:|---:|---:|
| score | **100** (100, 100, 100) | **100** (100, 100, 100) | **100** (100, 100, 100) |
| FCP [ms] | 1235 | 1171 | 751 |
| LCP [ms] | 1235 | 1171 | 751 |
| TBT [ms] | 74 | 0 | 30 |
| Speed Index [ms] | 1235 | 1171 | 751 |
| CLS | 0.000 | 0.000 | 0.000 |
| peso total [KB] | 49.0 | 36.2 | 30.2 | |
| Node | v26.12.0 |
| SO / CPU | Linux 7.2.15-1-omarchy / 11th Gen Intel(R) Core(TM) i7-11800H @ 2.30GHz |
| sveltekit | `http://localhost:5171` (`vite preview`, gzip, escuta só em `[::1]`) |
| astra | `http://127.0.0.1:5172` (servidor do esbuild, **sem compressão**) |
| fable | `http://127.0.0.1:5173` (`scripts/preview.js`, **sem compressão**) |

## Metodologia

- **Isolamento:** cada run usa um `BrowserContext` novo (cache HTTP e `localStorage`
  vazios, cache frio). Viewport 1280×720 nos dois perfis (só o throttling muda). Os 3 apps
  rodam em sequência, nunca em paralelo; cada bloco (medida × perfil × app, 10 runs) e
  cada run do Lighthouse roda segurando `flock /tmp/pragma-bench.lock` (helper `withLock`
  em `lib.mjs`: um filho `flock … cat` segura a trava até o bloco terminar), para não
  coincidir com as medições de build do outro agente.
- **Perfis** (aplicados via CDP na página antes da navegação):
  - **A**: sem throttling.
  - **B (mobile lento)**: `Emulation.setCPUThrottlingRate {rate: 4}` e
    `Network.emulateNetworkConditions {latency: 150, downloadThroughput: 200000 B/s
    (1,6 Mbps), uploadThroughput: 93750 B/s (750 Kbps)}`. Valores exatamente esses (o
    preset "Slow 4G" do DevTools aplica ainda fatores de ajuste; aqui não).
- **Script injetado** (`addInitScript`, idêntico nos 3): observadores de `longtask` e
  `largest-contentful-paint`, helpers de seleção independentes de implementação (por
  `aria-label`, texto, `h1`, `href`) e o medidor `arm()`.
- **Medidor de interação (`arm`)**: t0 = `event.timeStamp` do `pointerdown`/`keydown`
  real (input CDP do Playwright), capturado na `window`; um `MutationObserver` no
  `document` testa a condição a cada lote de mutações e registra `t_dom`; depois
  `requestAnimationFrame` + `MessageChannel` registra `t_frame` (≈ fim do frame seguinte,
  depois do paint). **As tabelas mostram `t_frame − t0`**; `t_dom` está em `perf.json`.
  É um proxy de INP (latência até o próximo frame), não o INP oficial.
- **Seletores**: links do `banner` por nome ("Painel", "Tarefas", "Loja"); Counter por
  `aria-label="Aumentar Cliques"` + `output` irmão; CartBadge = link `/loja` com texto
  "Carrinho"; tarefas = `main li input[type=checkbox]`; produtos = `main a[href^="/loja/"]`
  (9 no catálogo); StatCard por heading "Concluídas".

### O que cada medida faz

1. **Carga fria** de `/`, `/tarefas`, `/loja`: `goto` (espera `load`; na loja também os
   9 produtos), `networkidle` e +0,8 s (A) / +1,5 s (B). FCP do Paint Timing, LCP = última
   entrada do observador, DCL e `load` do Navigation Timing (`*EventEnd`), bytes =
   soma de `encodedDataLength` (CDP, inclui cabeçalhos), requisições = `requestWillBeSent`,
   bytes decodificados = Resource Timing. TBT = Σ(duração − 50) das long tasks que
   começam depois do FCP; também reporto a soma total das long tasks.
2. **TTI prático**: navega para `/` (`waitUntil: commit`) e entra num laço apertado: assim
   que `[aria-label="Aumentar Cliques"]` tem caixa, faz um clique real no centro; repete
   até o `output` sair de "0". Tempo = início da navegação → frame pintado com o novo
   valor. "cliques necessários" > 1 indicaria cliques perdidos antes do app ficar vivo.
3. **Navegação SPA** (um contexto por run, cache frio): carrega `/`, espera ociosidade, e
   mede clique → conteúdo: Painel→Tarefas (h1 + lista), Tarefas→Loja (h1 "Loja") e
   Tarefas→Loja até os 9 produtos (com e sem descontar os 400 ms simulados), Loja→detalhe
   (clique no nome "Café torrado 500 g"; link "Voltar" visível + 1 botão Adicionar),
   `history.back()` (t0 = `performance.now()` imediatamente antes; grade com 9 produtos
   de volta) e Loja→Painel. A 1ª ida à Loja inclui baixar o que faltar (chunk e
   `produtos.json`), pois o cache começa frio.
4. **Latência de interação**: Counter "+" no Painel; adicionar tarefa (Enter no campo,
   até a 4ª linha aparecer); marcar a 1ª tarefa (até o StatCard "Concluídas" mudar);
   digitar "s" na busca (4 → 1 item); "Adicionar" do 1º produto (até o texto do
   CartBadge mudar).
5. **Estresse (500 tarefas)**: semeadas no `localStorage` antes do app carregar, no
   formato de cada app (`mercadinho:tarefas` com `{id,titulo,feita}` no sveltekit;
   `mercadinho:tarefas` com `{id,title,done}` no fable; `mercadinho-astra-v1` com
   `{todos,cart,theme}` no astra). Títulos `Tarefa N par|impar`, pares concluídas. Mede:
   carga fria de `/tarefas` até 500 linhas pintadas; SPA Painel→Tarefas até 500 linhas;
   tecla "i" na busca (500→250); Backspace (250→500); filtro "Pendentes" (500→250).
6. **Memória**: após carregar `/` e após 1, 10 e 30 ciclos Painel→Tarefas→Loja→Painel
   (cada passo espera o conteúdo), roda `HeapProfiler.collectGarbage` 2× e lê
   `Performance.getMetrics` (`JSHeapUsedSize`, `Nodes`, `JSEventListeners`) + nº de
   elementos do `document`. O ciclo 1 aquece caches (catálogo, chunks), então o
   crescimento relevante é 1→30 e 10→30. Só no perfil A (throttling não muda o que fica
   retido; economiza ~10 min de trava).
7. **Lighthouse**: `lighthouse <url>/ --only-categories=performance` (preset mobile
   padrão, throttling **simulado**), com `--chrome-path` do Chromium do Playwright,
   3 runs por app, intercalados, cada um sob a trava.

## Resultados: perfil A (sem throttling)

### 1. Carga fria
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| / FCP [ms] | 212 (224) | 108 (116) | 108 (116) |
| / LCP [ms] | 212 (224) | 108 (116) | 108 (116) |
| / DOMContentLoaded [ms] | 70 (83) | 87 (92) | 72 (91) |
| / load [ms] | 71 (84) | 87 (92) | 75 (94) |
| / bytes transferidos [KB] | 49.0 | 36.2 | 30.0 |
| / bytes decodificados [KB] | 102.1 | 35.8 | 29.4 |
| / requisições | 18 | 3 | 3 |
| / long tasks (nº) | 1 | 1 | 1 |
| / long tasks (ms somados) [ms] | 57 (71) | 25 (55) | 26 (53) |
| / TBT (após FCP) [ms] | 0 (0) | 0 (0) | 0 (0) |
| /tarefas FCP [ms] | 184 (320) | 112 (128) | 100 (108) |
| /tarefas LCP [ms] | 184 (320) | 112 (128) | 100 (108) |
| /tarefas DOMContentLoaded [ms] | 77 (153) | 88 (94) | 66 (71) |
| /tarefas load [ms] | 78 (159) | 89 (96) | 79 (82) |
| /tarefas bytes transferidos [KB] | 49.4 | 36.2 | 30.0 |
| /tarefas bytes decodificados [KB] | 102.7 | 35.8 | 29.4 |
| /tarefas requisições | 18 | 3 | 3 |
| /tarefas long tasks (nº) | 0 | 0 | 0 |
| /tarefas long tasks (ms somados) [ms] | 0 (59) | 0 (51) | 0 (0) |
| /tarefas TBT (após FCP) [ms] | 0 (0) | 0 (0) | 0 (0) |
| /loja FCP [ms] | 182 (204) | 114 (120) | 98 (104) |
| /loja LCP [ms] | 594 (608) | 114 (120) | 500 (504) |
| /loja DOMContentLoaded [ms] | 52 (63) | 82 (97) | 65 (68) |
| /loja load [ms] | 53 (63) | 84 (98) | 77 (80) |
| /loja bytes transferidos [KB] | 51.1 | 37.2 | 31.2 |
| /loja bytes decodificados [KB] | 104.5 | 36.8 | 30.3 |
| /loja requisições | 21 | 4 | 4 |
| /loja long tasks (nº) | 0 | 1 | 0 |
| /loja long tasks (ms somados) [ms] | 0 (57) | 25 (55) | 0 (0) |
| /loja TBT (após FCP) [ms] | 0 (0) | 0 (0) | 0 (0) |

### 2. Tempo até interativo prático
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| TTI prático (nav → valor do + atualizado e pintado) [ms] | 123 (136) | 69 (92) | 79 (101) |
| 1º clique possível (botão com caixa) [ms] | 112 (126) | 56 (62) | 66 (69) |
| cliques necessários | 1 | 1 | 1 |

### 3. Navegação SPA (clique → conteúdo pintado)
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| Painel → Tarefas [ms] | 31 (31) | 14 (15) | 14 (15) |
| Tarefas → Loja (conteúdo/h1) [ms] | 31 (31) | 15 (15) | 14 (15) |
| Tarefas → Loja (produtos visíveis) [ms] | 435 (436) | 420 (420) | 419 (420) |
| Tarefas → Loja (produtos − 400 ms simulados) [ms] | 35 (36) | 20 (20) | 19 (20) |
| Loja → detalhe do produto [ms] | 31 (31) | 15 (15) | 15 (15) |
| detalhe → Loja (history.back) [ms] | 15 (15) | 10 (10) | 9 (9) |
| Loja → Painel [ms] | 31 (32) | 15 (15) | 15 (16) |

### 4. Latência de interação (input → próximo frame)
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| Counter + (Painel) [ms] | 13 (13) | 13 (13) | 13 (14) |
| adicionar tarefa (Enter) [ms] | 6 (7) | 5 (5) | 4 (4) |
| marcar tarefa (checkbox → StatCard) [ms] | 15 (15) | 15 (15) | 15 (15) |
| digitar 1 char na busca (4 → 1 item) [ms] | 9 (10) | 10 (11) | 10 (11) |
| Adicionar ao carrinho → CartBadge [ms] | 16 (16) | 15 (15) | 15 (16) |

### 5. Estresse com 500 tarefas
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| carga fria /tarefas até 500 linhas pintadas [ms] | 206 (215) | 141 (148) | 135 (143) |
| carga fria /tarefas LCP [ms] | 216 (224) | 154 (164) | 144 (156) |
| SPA Painel → Tarefas até 500 linhas [ms] | 135 (137) | 86 (90) | 76 (78) |
| busca: digitar "i" (500 → 250) [ms] | 31 (33) | 27 (29) | 24 (25) |
| busca: Backspace (250 → 500) [ms] | 73 (74) | 52 (56) | 49 (49) |
| filtro: Pendentes (500 → 250) [ms] | 40 (41) | 34 (36) | 29 (30) |

### 6. Memória e vazamento (perfil A)
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| heap JS após carregar / (MB) [MB] | 1.36 | 0.79 | 0.86 |
| heap após 1 ciclo (MB) [MB] | 3.10 | 2.24 | 2.33 |
| heap após 10 ciclos (MB) [MB] | 3.52 | 2.41 | 2.51 |
| heap após 30 ciclos (MB) [MB] | 3.73 | 2.52 | 2.58 |
| Δ heap 1 → 30 ciclos (KB) [KB] | 645.0 | 294.4 | 257.1 |
| Δ heap 10 → 30 ciclos (KB) [KB] | 213.2 | 116.2 | 73.1 |
| nós (CDP Nodes) inicial | 486 | 167 | 177 |
| nós (CDP Nodes) após 1 / 30 ciclos | 670 / 670 | 170 / 170 | 183 / 183 |
| elementos no document, inicial / após 30 | 115 / 123 | 103 / 103 | 95 / 95 |
| JSEventListeners inicial | 15 | 20 | 17 |
| JSEventListeners após 1 / 30 ciclos | 48 / 48 | 46 / 46 | 43 / 43 |

## Resultados: perfil B (CPU 4× + Slow 4G)

### 1. Carga fria
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| / FCP [ms] | 1118 (1128) | 760 (832) | 646 (688) |
| / LCP [ms] | 1118 (1128) | 760 (832) | 646 (688) |
| / DOMContentLoaded [ms] | 426 (435) | 729 (797) | 595 (626) |
| / load [ms] | 428 (436) | 729 (798) | 603 (634) |
| / bytes transferidos [KB] | 49.0 | 36.2 | 30.0 |
| / bytes decodificados [KB] | 102.1 | 35.8 | 29.4 |
| / requisições | 18 | 3 | 3 |
| / long tasks (nº) | 2 | 1 | 1 |
| / long tasks (ms somados) [ms] | 344 (384) | 184 (241) | 120 (158) |
| / TBT (após FCP) [ms] | 0 (0) | 0 (0) | 0 (0) |
| /tarefas FCP [ms] | 964 (1012) | 782 (896) | 686 (692) |
| /tarefas LCP [ms] | 964 (1012) | 782 (896) | 686 (692) |
| /tarefas DOMContentLoaded [ms] | 431 (443) | 737 (813) | 614 (620) |
| /tarefas load [ms] | 433 (446) | 749 (845) | 657 (662) |
| /tarefas bytes transferidos [KB] | 49.4 | 36.2 | 30.0 |
| /tarefas bytes decodificados [KB] | 102.7 | 35.8 | 29.4 |
| /tarefas requisições | 18 | 3 | 3 |
| /tarefas long tasks (nº) | 2 | 1 | 1 |
| /tarefas long tasks (ms somados) [ms] | 188 (233) | 211 (259) | 167 (170) |
| /tarefas TBT (após FCP) [ms] | 0 (0) | 0 (0) | 0 (0) |
| /loja FCP [ms] | 1144 (1176) | 728 (764) | 678 (684) |
| /loja LCP [ms] | 1704 (1732) | 728 (764) | 1236 (1244) |
| /loja DOMContentLoaded [ms] | 433 (446) | 690 (720) | 611 (616) |
| /loja load [ms] | 435 (448) | 697 (728) | 650 (657) |
| /loja bytes transferidos [KB] | 51.1 | 37.2 | 31.2 |
| /loja bytes decodificados [KB] | 104.5 | 36.8 | 30.3 |
| /loja requisições | 21 | 4 | 4 |
| /loja long tasks (nº) | 2 | 1 | 1 |
| /loja long tasks (ms somados) [ms] | 287 (331) | 167 (189) | 154 (158) |
| /loja TBT (após FCP) [ms] | 0 (0) | 0 (0) | 0 (0) |

### 2. Tempo até interativo prático
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| TTI prático (nav → valor do + atualizado e pintado) [ms] | 1035 (1049) | 744 (752) | 711 (716) |
| 1º clique possível (botão com caixa) [ms] | 1006 (1019) | 723 (727) | 686 (693) |
| cliques necessários | 1 | 1 | 1 |

### 3. Navegação SPA
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| Painel → Tarefas [ms] | 282 (289) | 61 (67) | 61 (63) |
| Tarefas → Loja (conteúdo/h1) [ms] | 266 (276) | 55 (56) | 40 (41) |
| Tarefas → Loja (produtos visíveis) [ms] | 858 (866) | 488 (493) | 626 (627) |
| Tarefas → Loja (produtos − 400 ms simulados) [ms] | 458 (466) | 88 (93) | 226 (227) |
| Loja → detalhe do produto [ms] | 203 (215) | 40 (42) | 34 (35) |
| detalhe → Loja (history.back) [ms] | 57 (61) | 45 (48) | 40 (41) |
| Loja → Painel [ms] | 57 (61) | 36 (39) | 30 (31) |

### 4. Latência de interação
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| Counter + (Painel) [ms] | 25 (26) | 20 (22) | 17 (21) |
| adicionar tarefa (Enter) [ms] | 25 (26) | 21 (23) | 16 (16) |
| marcar tarefa (checkbox → StatCard) [ms] | 19 (20) | 20 (21) | 16 (17) |
| digitar 1 char na busca (4 → 1 item) [ms] | 21 (21) | 20 (21) | 18 (18) |
| Adicionar ao carrinho → CartBadge [ms] | 36 (40) | 30 (32) | 22 (25) |

### 5. Estresse com 500 tarefas
| Métrica | sveltekit | astra | fable |
|---|---:|---:|---:|
| carga fria /tarefas até 500 linhas pintadas [ms] | 1325 (1474) | 1075 (1125) | 1008 (1036) |
| carga fria /tarefas LCP [ms] | 1324 (1464) | 1082 (1128) | 1008 (1036) |
| SPA Painel → Tarefas até 500 linhas [ms] | 728 (800) | 411 (445) | 371 (387) |
| busca: digitar "i" (500 → 250) [ms] | 161 (166) | 129 (133) | 110 (115) |
| busca: Backspace (250 → 500) [ms] | 353 (369) | 253 (305) | 274 (287) |
| filtro: Pendentes (500 → 250) [ms] | 204 (208) | 161 (174) | 132 (134) |

## 7. Lighthouse (mobile padrão, `/`)
| Lighthouse (mediana de 3; valores individuais do score) | sveltekit | astra | fable |
|---|---:|---:|---:|
| score | **100** (100, 100, 100) | **100** (100, 100, 100) | **100** (100, 100, 100) |
| FCP [ms] | 1235 | 1171 | 751 |
| LCP [ms] | 1235 | 1171 | 751 |
| TBT [ms] | 74 | 0 | 30 |
| Speed Index [ms] | 1235 | 1171 | 751 |
| CLS | 0.000 | 0.000 | 0.000 |
| peso total [KB] | 49.0 | 36.2 | 30.2 |

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
