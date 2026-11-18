# Desempenho no browser: sveltekit × astra × fable

Gerado por `bench/perf/` (Playwright + Chromium) em {{generatedAt}}. Dados brutos e
medianas em [`perf.json`](perf.json). Para reproduzir: `cd bench/perf && npm i &&
node bench.mjs load tti nav interact stress memory && node lighthouse.mjs 3 && node report.mjs`.

**Leitura das tabelas:** app nas colunas; tempos em ms como `mediana (p90)` de 10 runs;
contagens e bytes são a mediana (são determinísticos). Menor é melhor, exceto o score do
Lighthouse.

{{summary}}

## Ambiente e versões

| Item | Valor |
|---|---|
| Chromium | {{chromium}} (Chromium do Playwright, `channel: 'chromium'`, headless novo) |
| Playwright | {{playwright}} |
| Lighthouse | {{lighthouse}} |
| Node | {{node}} |
| SO / CPU | {{os}} / {{cpu}} |
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
{{A.load}}

### 2. Tempo até interativo prático
{{A.tti}}

### 3. Navegação SPA (clique → conteúdo pintado)
{{A.nav}}

### 4. Latência de interação (input → próximo frame)
{{A.interact}}

### 5. Estresse com 500 tarefas
{{A.stress}}

### 6. Memória e vazamento (perfil A)
{{A.memory}}

## Resultados: perfil B (CPU 4× + Slow 4G)

### 1. Carga fria
{{B.load}}

### 2. Tempo até interativo prático
{{B.tti}}

### 3. Navegação SPA
{{B.nav}}

### 4. Latência de interação
{{B.interact}}

### 5. Estresse com 500 tarefas
{{B.stress}}

## 7. Lighthouse (mobile padrão, `/`)
{{lighthouse}}

{{anomalies}}
