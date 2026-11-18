# tests_1: comparação consolidada (sveltekit × astra × fable)

Consolida, sem medições novas: `SPEC.md`, `TEMPOS.md`, `bench/static.md` + `static.json`,
`bench/perf.md` + `perf.json` e os três `RESULTADO.md`. Todos os números vêm dessas fontes.
Onde faltava dado, aparece "—".

As três pastas implementam o mesmo app ("Mercadinho": SPA com 3 páginas, 404, `/loja/:id`,
estado global persistido e filtros na URL):

- **sveltekit**: Svelte 5 (runes) e SvelteKit 2, com `ssr = false` e adapter-static. É a referência de mercado.
- **astra**: JS nativo sem signals, com store central, `dispatch` e componentes `mount`/`update`/`destroy`.
- **fable**: JS nativo com signals, `effect`/`scope` e custom elements.

## Tabela principal

Convenções:

- **Negrito** marca o melhor da linha. Negrito em mais de uma coluna quer dizer empate, ou diferença dentro do ruído (quantização de frame de 16,7 ms no perfil A, ou a dispersão das 5 execuções de build e dev).
- Linhas sem negrito não têm um "melhor" objetivo.
- Menor é melhor, exceto no score do Lighthouse.
- Os perfis de performance usam a mediana de 10 runs. **A** roda sem throttling. **B** roda com CPU 4×, 150 ms de RTT e 1,6 Mbps.

| Seção | Medida | sveltekit | astra | fable |
|---|---|---:|---:|---:|
| Desenvolvimento | Tempo do agente | 21 min 45 s | **15 min 38 s** | 22 min 47 s |
| | Chamadas de ferramenta¹ | 37 | 56 | **33** |
| | Modelo | claude opus | gpt-6-astra | claude-fable-5-2 |
| Código | App: arquivos / linhas de código² | 32 / 1 510 | **10 / 828** | 27 / 1 195 |
| | App: bytes de fonte | 44 893 | 45 635 | **41 766** |
| | Runtime próprio / tooling próprio (linhas de código) | **0 / 0** | 109 / 22 | 323 / 107 |
| Bundle | 1ª visita a `/`: gzip / brotli (kB) | 41,5 / 37,1 | 12,3 / 10,9 | **10,8 / 9,6** |
| | 1ª visita a `/`: arquivos | 17 | **3** | **3** |
| | JS total: gzip / brotli (kB) | 45,6 / 40,8 | 8,2 / 7,4 | **7,2 / 6,5** |
| | CSS total: gzip / brotli (kB) | 5,4 / 4,4 | 3,7 / 3,3 | **3,3 / 2,8** |
| | Arquivos JS + CSS no build | 19 + 8 | 1 + 1 | 1 + 1 |
| Dependências | Diretas (dev + prod) | 7 + 0 | **1 + 0** | **1 + 0** |
| | node_modules: pacotes / MB | 58 / 73,8 | **2** / 11,3 | **2 / 10,5** |
| Ferramenta (s) | Build de produção (mediana de 5) | 6,01 | **0,34** | **0,33** |
| | Dev: cold start até o 1º 200 (mediana de 5) | 2,60 | **0,38** | **0,40** |
| | `npm ci`, cache quente (1×) | 3,81 | **0,96** | 1,15 |
| Perf. A (ms) | `/` FCP = LCP, carga fria | 212 | **108** | **108** |
| | TTI prático em `/` | 123 | **69** | 79 |
| | SPA Painel→Tarefas / Tarefas→Loja (produtos − 400 ms) | 31 / 35 | **14 / 20** | **14 / 19** |
| | Counter + / Adicionar→CartBadge | **13 / 16** | **13 / 15** | **13 / 15** |
| | 500 tarefas: SPA até a lista | 135 | 86 | **76** |
| | 500 tarefas: tecla "i" / Backspace / filtro | 31 / 73 / 40 | 27 / 52 / 34 | **24 / 49 / 29** |
| Perf. B (ms) | `/` FCP = LCP, carga fria | 1 118 | 760 | **646** |
| | TTI prático em `/` | 1 035 | 744 | **711** |
| | Soma das long tasks em `/` | 344 | 184 | **120** |
| | SPA Painel→Tarefas / Tarefas→Loja (produtos − 400 ms)³ | 282 / 458 | **61 / 88** | **61** / 226 |
| | Counter + / Adicionar→CartBadge | 25 / 36 | 20 / 30 | **17 / 22** |
| | 500 tarefas: SPA até a lista | 728 | 411 | **371** |
| | 500 tarefas: tecla "i" / Backspace / filtro | 161 / 353 / 204 | 129 / **253** / 161 | **110** / 274 / **132** |
| Memória (perfil A) | Heap após 30 ciclos (MB) / Δ heap 10→30 (KB) | 3,73 / 213 | **2,52** / 116 | 2,58 / **73** |
| | Vazamento de nós DOM ou listeners (ciclos 1→30) | **nenhum** | **nenhum** | **nenhum** |
| Lighthouse (mobile, `/`) | Score / FCP (ms) / TBT (ms) | **100** / 1 235 / 74 | **100** / 1 171 / **0** | **100 / 751** / 30 |
| Qualidade | Checks e testes no repositório | svelte-check ✅ (0 erros, 0 avisos); e2e Playwright | nenhum check; e2e Playwright | `npm test` ✅ 22/22 (runtime); lint do dialeto ✅; e2e fora do repo |
| | Critérios de aceite (autodeclarados no RESULTADO) | **10/10** | **10/10** | **10/10** |
| | `console.*`: app / runtime / tooling | **0 / 0 / 0** | **0 / 0 / 0** | **0 / 0** / 3 |

Notas:

1. As chamadas de ferramenta só se comparam de forma aproximada. Os harnesses são diferentes, e os três agentes escreveram a maior parte dos arquivos por heredoc no shell, então uma chamada pode criar vários arquivos.
2. "Linhas de código" exclui linhas em branco e linhas só de comentário. O astra tem metade das linhas do sveltekit e quase os mesmos bytes, porque suas linhas são longas. Por isso, leia linhas e bytes juntos.
3. Em Tarefas→Loja no perfil B, o astra faz o fetch em paralelo com o atraso de 400 ms, enquanto os outros dois esperam o atraso e só depois buscam. Isso dá ao astra cerca de 140 ms de vantagem. É uma diferença de implementação, não da abordagem.

Outros dados:

- **Bytes transferidos na carga de `/`** (perf, CDP, com cabeçalhos): 49,0 / 36,2 / 30,0 KB. O sveltekit foi servido com gzip; astra e fable, sem compressão (ver Limitações).
- **Fable e Firefox:** o fable também verificou os critérios de aceite no Firefox. Os outros dois, só no Chromium.

## Leitura

- **Bundle:** os nativos entregam cerca de 4× menos na 1ª visita: 10,8 a 12,3 kB gzip contra 41,5 kB, em 3 arquivos contra 17. No sveltekit, cerca de 30 kB desse JS são framework (runtime do Svelte e router do Kit) e só cerca de 7 kB são o app. Entre astra e fable a diferença é de 1,5 kB gzip, irrelevante na prática.
- **Carga fria:** é onde está a maior diferença. No perfil A, os nativos pintam em cerca de metade do tempo (108 contra 212 ms). No B, pintam 32% a 42% antes (646 e 760 contra 1 118 ms) e ficam interativos de 290 a 320 ms antes. A causa principal é a cascata de cerca de 18 módulos do SvelteKit em modo SPA (`ssr = false`) com RTT de 150 ms, mais do que os bytes.
- **Interações pequenas não diferenciam no perfil A.** Counter e CartBadge medem 13 a 16 ms nos três, presos ao frame. No B a ordem é fable < astra < sveltekit, com diferenças de 3 a 14 ms, pequenas para o usuário.
- **Navegação SPA:** no B, o sveltekit gasta cerca de 280 ms na 1ª visita a cada rota, porque baixa o chunk da rota com cache frio. Os nativos levam 61 ms. Em rotas já visitadas a diferença cai para 10 a 20 ms. O 88 contra 226 ms entre astra e fable em Tarefas→Loja vem do fetch em paralelo do astra, não de signals contra `dispatch`.
- **Estresse com 500 tarefas:** o fable é o mais rápido em quase tudo, e o sveltekit fica 30% a 100% atrás dos nativos. Entre astra e fable, a distância é de 3 a 10 ms no A, abaixo de um frame, e de 20 a 40 ms no B, onde o astra ganha no Backspace. Neste tamanho, signals contra store + `dispatch` quase não pesam.
- **Ferramenta:** com esbuild, o build é cerca de 18× mais rápido (0,33 a 0,34 s contra 6,01 s) e o dev sobe cerca de 6,5× mais rápido (0,38 a 0,40 s contra 2,60 s). O node_modules cai de 58 pacotes (75 MB) para 2 (11 MB). Astra e fable empatam dentro do ruído.
- **Memória e Lighthouse não separam os apps.** Nenhum dos três vaza nós ou listeners, e o crescimento do heap desacelera, o que é compatível com aquecimento. O heap dos nativos é cerca de 1/3 menor. O Lighthouse dá 100 nos três e só as métricas brutas variam.
- **Desenvolvimento e qualidade:** o astra terminou mais rápido (15 min 38 s), com menos linhas e o menor runtime (109 linhas), mas sem typecheck, lint nem teste unitário. O fable levou praticamente o mesmo tempo que o sveltekit e entregou 3× mais runtime próprio, um lint do dialeto e 22 testes. Como há um app por modelo, essas diferenças não separam o efeito da abordagem do efeito do modelo.

## Limitações

- **n = 1, e cada app foi feito por um modelo e um harness diferentes.** Tempo, chamadas, volume de código e decisões misturam abordagem, modelo e agente. Não dá para atribuir "15 min contra 22 min" à abordagem.
- **Compressão desigual no perf.** O `vite preview` (sveltekit) serve com gzip. O servidor do esbuild (astra) e o `preview.js` (fable) servem sem compressão. Isso favorece o sveltekit em bytes transferidos e na carga do perfil B.
- **O sveltekit rodou como SPA (`ssr = false`).** Esse não é o modo em que ele é mais forte. SSR ou prerender mudariam o FCP e o LCP, e isso não foi medido. Também por causa disso, o DOMContentLoaded e o `load` do sveltekit não se comparam com os dos outros dois.
- **Diferenças de implementação entram nas medidas:**
  - o astra faz o fetch em paralelo com o atraso;
  - o LCP de `/loja` no astra é o texto "Carregando produtos…";
  - o filtro do fable usa links, e os outros usam botões;
  - o astra mostra mais texto por página.
- **Quantização e métricas de interação.** No perfil A, diferenças menores que cerca de 16,7 ms não significam nada. A latência medida é um proxy de INP, não o INP oficial. O TBT dá 0 em todos, porque a long task termina antes do FCP.
- **Memória:** só foi medida no perfil A, sem heap snapshot. A conclusão "sem vazamento" se apoia em contagens de nós e listeners.
- **Lighthouse:** usa throttling simulado e o preset mobile (412×823), mede só `/` e roda 3 vezes. O perfil B usa viewport desktop.
- **1ª visita:** os bytes vêm de análise estática do build, não de captura de rede. O preload por hover do SvelteKit não entra na conta. O sveltekit mediu 37 289 B de JS no próprio RESULTADO, com Playwright, contra 41,5 kB de JS + CSS + HTML no bench.
- **Ambiente:** uma máquina, sem isolamento de CPU, com outros agentes rodando. A trava `flock` impediu sobreposição entre os benchmarks, mas não controlou o resto. Os tempos de build e dev têm ruído de alguns décimos de segundo. O `npm ci` rodou uma vez só.
- **Contagem de linhas:** é léxica e simples. Astra e fable usam versões diferentes do esbuild (0,27 e 0,25).
- **Não medido:**
  - clareza e manutenção por agentes, que são parte do objetivo da SPEC (nenhuma tarefa de alteração foi feita depois);
  - custo em tokens;
  - Safari e Firefox na performance, que só rodou no Chromium;
  - acessibilidade com leitor de tela;
  - escala além de 500 tarefas e 9 produtos;
  - chunking por rota nos nativos, que usam um bundle único.
- **Critérios de aceite:** os 10/10 são autodeclarados pelos agentes, com scripts próprios. O bench não os reverificou.

## Nota final: o que serve para o tests_2

O `core/DECISOES.md` (seção "HTML: autoria em `.html` e páginas pré-geradas") muda a direção:

- a marcação vai em arquivos `.html`, sem HTML dentro de JS;
- as páginas podem ser pré-geradas no build;
- o JS só se liga a nós que já existem, sem re-render.

Astra e fable montam todo o HTML em JS a partir de um `index.html` vazio, e o pragma não vai seguir esse caminho. A seção "Performance" também pede chunk por rota, enquanto astra e fable usam bundle único.

**Continuam úteis:**

- **sveltekit como linha de base.** Bundle, perfis A e B, memória e ferramenta continuam valendo como "referência de mercado". Vale acrescentar uma variante com prerender ou SSR, porque o tests_2 vai pré-gerar HTML e compará-lo com o SvelteKit em SPA seria injusto com ele.
- **Metodologia e scripts do bench.** Valem `bench/perf` (perfis A e B, medidor `arm`, seletores por papel e texto, estresse com 500 tarefas, ciclos de memória, `flock`) e o método estático (gzip e brotli por arquivo, 1ª visita, linhas, build, dev e `npm ci`). Para reaproveitar, o tests_2 precisa manter os `aria-label` e textos da SPEC e documentar o formato do `localStorage`. Precisa também servir os três com a mesma compressão.
- **Ferramenta como meta para o CLI Rust.** Build em cerca de 0,33 s, dev em cerca de 0,4 s e 1 dependência de desenvolvimento são o patamar que o `pragma build`/`dev` precisa igualar ou superar.
- **Teto de JS.** Os nativos ficaram em 7,2 a 8,2 kB gzip de JS total, e o runtime do fable sozinho em 2,3 kB. Com HTML pré-gerado o JS deveria cair, então esses números servem de teto.
- **Interação, estresse e memória dos nativos.** Contador, carrinho, busca, filtro em 500 tarefas, heap e ausência de vazamento servem de referência para a camada de comportamento, porque o estado e as atualizações continuam existindo. As listas vão passar a usar `<template>` + `cloneNode`, então esses números são alvo a bater, não previsão.
- **SPEC, critérios de aceite e `preflight.css`.** Podem ser reaproveitados quase sem mudança.

**Deixam de valer, ou só servem como "antes":**

- **FCP, LCP, TTI e long tasks de carga fria de astra e fable.** Medem renderização no cliente com o JS no caminho crítico. Com HTML pré-gerado, a primeira tela depende só do HTML e do CSS, então esses números não preveem nada. Servem apenas como o ponto de partida a superar.
- **Distribuição de código de astra e fable.** Linhas por tipo, `.js` contra `.html`, e partes do runtime como `h()`, a região reativa e os templates de `common.js` deixam de existir no dialeto. Só o volume total, em linhas e bytes, continua comparável.
- **1ª visita com 3 arquivos e "1 + 1 arquivos no build".** O chunk por rota muda isso por decisão. O número atual serve só para medir quanto o chunking custa ou ganha, como o próprio DECISOES pede.
- **Navegação SPA e heap entre rotas.** Dependem da decisão em aberto entre MPA real e router que troca HTML pré-gerado. Em MPA, as medidas de navegação e de memória entre rotas mudam de natureza.
- **Tempo do agente e chamadas de ferramenta.** n = 1, modelos diferentes e um dialeto que ainda não existia. Não servem para prever o tests_2.
- **Score do Lighthouse.** Deu 100 nos três e não discrimina nada. Se for usado, só as métricas brutas.
