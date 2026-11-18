# RESULTADO: `fable/` (nativo, com signals)

Os 10 critérios de aceite passam no Chromium e no Firefox, no dev e no preview do build.
O bundle de produção tem 7.152 bytes de JS e 3.265 bytes de CSS (gzip), em um arquivo de cada.

## 1. Como rodar

Requer Node 20 ou mais novo (testado no 26.12).

```sh
npm install        # instala só o esbuild
npm run dev        # dev em http://127.0.0.1:5173 (rebuild a cada request, sourcemaps)
npm run build      # build de produção em dist/
npm run preview    # serve dist/ em http://127.0.0.1:5173 com fallback para index.html
npm test           # 22 testes do runtime de reatividade (node --test, sem dependências)
npm run check      # lint do dialeto (regex, sem dependências)
```

Use `127.0.0.1`, não `localhost`. Nesta máquina a porta 5173 em `[::1]` já estava ocupada por um `vite dev` de outro projeto, que não toquei; por isso dev e preview escutam só em `127.0.0.1`. `HOST` e `PORT` mudam o endereço.

## 2. Medidas (build de produção)

| Medida | Valor |
|---|---|
| JS total, gzip | **7.152 bytes** (1 arquivo, 18.811 bytes minificado) |
| JS na primeira visita a `/`, gzip | **7.152 bytes** (o mesmo arquivo; não há code splitting) |
| CSS total, gzip | **3.265 bytes** (1 arquivo, 10.587 bytes; inclui o preflight) |
| Linhas do app, `.js` | 834 (25 arquivos em `src/`, fora de `src/lib/`) |
| Linhas do app, `.css` | 534 (`src/styles/app.css`, sem o `preflight.css`) |
| Linhas do app, `.html` | 20 (`index.html`) |
| Linhas do runtime próprio | 448 (`src/lib/`), das quais 323 sem comentários e linhas em branco |

O runtime se divide assim:

| Arquivo | Linhas | Papel |
|---|---|---|
| `src/lib/reactive.js` | 239 | `signal`, `computed`, `effect`, `batch`, `scope`, `onCleanup`, `untrack` |
| `src/lib/dom.js` | 106 | `h` (elementos e bindings), região reativa, `list` (lista com chave) |
| `src/lib/router.js` | 69 | History API, `path`/`search`, `query`, `routeView` |
| `src/lib/component.js` | 34 | `define` (custom element em light DOM) |

Sozinho, o runtime minificado tem 4.848 bytes (2.296 gzip): cerca de um terço do JS.
Fora das contagens acima ficam os testes do runtime (324 linhas) e os scripts de build, preview e lint (126 linhas).

## 3. Critérios de aceite

Verificados com Playwright (Chromium 1255 e Firefox 159, headless), instalado em `/tmp/fable-pw`, fora do projeto.
O roteiro tem 9 cenários, um por critério de 1 a 9, e rodou contra o preview do build e contra o dev: 18 de 18 passaram em cada servidor, sem erro no console.

| # | Critério | | Como foi verificado |
|---|---|---|---|
| 1 | 3 páginas, 404 e `/loja/:id`; navegação sem reload | ✅ | Cliques nos links do header, no nome do produto e em "Voltar". Uma variável em `window` criada antes sobrevive a todas as navegações. Confere `<h1>`, `<title>` e `aria-current` em cada rota. |
| 2 | Voltar/avançar e deep link, com filtros na URL | ✅ | Abre `/tarefas?filtro=concluidas` e `/loja?categoria=limpeza` direto; troca de filtro, `goBack`/`goForward` e `reload` conferindo URL e lista; refresh em `/loja/cafe`. |
| 3 | `Counter`s respeitam `min`/`max` e são independentes | ✅ | "Pulos de 5" para em 50 e em 0 com o botão desabilitado; "Cliques" vai a −1; "Na aba" muda sem afetar os outros e mantém o valor ao trocar de aba. Na loja, a quantidade de um card não afeta os outros, o mínimo é 1 e o máximo é o estoque. |
| 4 | Carrinho atualiza `CartBadge` e `StatCard`s | ✅ | Badge lido logo após cada "Adicionar" (2, depois 5). No Painel: 5 itens, R$ 79,30, tom `ok`; com carrinho vazio, tom `alerta`. Também −/+, remover, esvaziar e o limite de estoque no `CartSummary`. |
| 5 | Tarefas: adicionar, validar, concluir, editar, remover, buscar, filtrar, limpar | ✅ | As 4 validações (vazio, curto, 81 caracteres, duplicata sem diferenciar maiúsculas); Enter envia, limpa e devolve o foco; edição com Enter e Esc; busca; os 3 filtros; "Nenhuma tarefa"; confirmação com Cancelar e Confirmar. |
| 6 | Refresh mantém tarefas, carrinho e tema | ✅ | `reload` depois de criar tarefa, concluir outra, adicionar ao carrinho e ligar o tema escuro; confere `data-theme`, badge, total e lista. |
| 7 | `Clock` para ao sair do Painel, sem vazamento | ✅ | `setInterval`/`clearInterval` instrumentados: 1 timer ativo no Painel, 0 fora, nenhum tick em 2,2 s fora, 1 ao voltar depois de 10 idas e vindas. Effects: uma cópia instrumentada em `/tmp` contou effects e computeds vivos em 26 voltas por todas as páginas e a contagem ficou idêntica (47 / 71 / 107 / 22). |
| 8 | Esqueleto, erro com `?falhar=1` e cache | ✅ | 6 esqueletos antes dos 9 cards; `?falhar=1` mostra o erro depois de 400 ms ou mais, sem fetch; "Tentar de novo" carrega. Requisições a `/produtos.json` contadas: 1 mesmo depois de sair, voltar e abrir `/loja/cafe`. |
| 9 | Modal fecha com Esc e devolve o foco; Tabs por teclado | ✅ | Esc, botão Fechar e clique no fundo fecham, e `document.activeElement` volta a ser "Sobre o app"; clique dentro não fecha. ←, → (com volta), Home; confere `aria-selected`, `tabindex` e `aria-controls`. |
| 10 | Build estático funciona com fallback para `index.html` | ✅ | Os cenários de 1 a 9 rodaram contra `dist/` servido por `scripts/preview.js`, incluindo os deep links. |

Não foi verificado:

- Safari/WebKit e leitores de tela.
- O visual só foi conferido por screenshots (claro, escuro e 390 px de largura), sem teste automatizado.
- `list` com reordenação não ocorre neste app (os filtros só removem e inserem). Testei à parte, nos dois browsers, com um script em `/tmp`: reordenar, inserir no meio, atualizar e remover preservam nós e estado. Esse teste não está na suíte do repositório.

## 4. Decisões de arquitetura

### Onde cada conceito vive

| Conceito | Onde |
|---|---|
| Estado global | `src/state/todos.js:6`, `src/state/cart.js:8`, `src/state/theme.js:7`, `src/state/products.js` (signals de módulo; só as funções exportadas escrevem) |
| Estado global derivado | `src/state/todos.js:12-14`, `src/state/cart.js:10-11` |
| Persistência | `src/state/persisted.js:4` (signal mais effect global que grava em `localStorage`) |
| Estado local | `src/components/counter.js:7-8` (`raw` mais `value` derivado com o clamp) |
| Estado na URL | `src/lib/router.js:5-6` (`path`, `search`), `src/lib/router.js:36` (`query`), `src/components/filter-bar.js:9` |
| Reatividade | `src/lib/reactive.js:140` (`signal`), `:166` (`computed`), `:181` (`effect`), `:188` (`batch`), `:205` (`scope`) |
| Binding no DOM | `src/lib/dom.js:23` (`h`), `src/lib/dom.js:48` (região reativa) |
| Componente | `src/lib/component.js:11` (`define`); exemplo em `src/components/counter.js` |
| Rotas | `src/lib/router.js:53` (`routeView`); tabela em `src/main.js:10` |
| Efeito com limpeza | `src/components/clock.js:7-8` (`setInterval` mais `onCleanup`); descarte em `src/lib/component.js:26` |
| Lista com chave | `src/lib/dom.js:79` (`list`); usos em `src/components/todo-list.js:63`, `cart-summary.js:29`, `product-grid.js:28` |
| Dados assíncronos | `src/state/products.js` (`loadProducts`); estados de carga em `src/components/product-grid.js:15` |

### Decisões

- **Reatividade push-pull em três estados.** Uma escrita só marca os dependentes; o valor é recalculado na leitura ou no flush. Isso dá `computed` memoizado e sem glitch em diamante, e não recalcula um ramo condicional que deixou de valer.
- **Effects rodam na ordem de criação.** O pai sempre roda antes dos filhos, então um effect de uma página ou linha já removida nunca chega a rodar com dado velho.
- **Escrever em signal dentro de effect ou computed lança erro** (`src/lib/reactive.js:148`). A regra é do runtime, não do lint. Ela pegou um bug real durante a verificação: o Chromium dispara `blur` de forma síncrona quando o campo focado é ocultado dentro de um effect, e o handler escrevia em um signal. Por isso os handlers registrados por `h` rodam com `untrack` (`src/lib/dom.js:30`).
- **O app não escreve no DOM.** `h` monta a estrutura e transforma toda prop ou filho que é função em um effect. "Escrita no DOM só dentro de effect" vira propriedade da biblioteca, e `npm run check` barra a escrita direta em `src/` fora de `src/lib/`. Há uma exceção marcada com `// dom-ok` (`src/state/theme.js:14`, dentro de um effect global).
- **Props são valores ou getters.** O filho lê com `read(prop)` dentro dos seus bindings e passa a depender direto do signal do pai. Não há cópia de prop para signal, que seria uma escrita dentro de effect.
- **Componente é uma função `(props, children, host)`** registrada como custom element em light DOM. Ela roda a cada conexão, dentro de um `scope` descartado no `disconnectedCallback`. O CSS é global, organizado nas camadas pedidas.
- **Composição pelos filhos.** `Modal` e `Tabs` recebem o conteúdo como filhos do elemento; `Tabs` lê o rótulo de cada painel em `data-tab` e oculta os painéis inativos sem desmontar.
- **`list` dá a cada linha um getter do item.** A linha é criada uma vez por chave e se atualiza no lugar, então o estado de edição de uma tarefa sobrevive às mudanças da lista. Para mover, usa `moveBefore` quando existe, o que preserva o estado do custom element.
- **Router sobre a History API**, com um único listener de clique para links internos. `routeView` reage só ao `path`: trocar o filtro na URL não remonta a página.
- **`FilterBar` é uma lista de links.** Voltar/avançar, refresh e "abrir em nova aba" funcionam sem código próprio.
- **Um bundle só, sem code splitting.** Com 7 KB de JS no total, dividir por rota custaria mais em requisições do que economizaria em bytes.
- **Arquivos com hash de conteúdo** no build; o `index.html` é reescrito com os nomes finais.

### Desvios e escolhas onde a spec não decide

- **`?falhar=1` força o erro só na carga automática.** "Tentar de novo" faz uma tentativa real e carrega os produtos. Assim o caminho de recuperação pode ser testado; se o botão também falhasse, o erro seria permanente com esse parâmetro.
- **O runtime tem mais do que a lista pedida**: `h`, a região reativa e `untrack`. Sem `h`, cada componente teria de criar seus effects à mão, e a regra do DOM dependeria de disciplina.
- **A regra do DOM tem um limite assumido.** Comandos imperativos que não são renderização de estado ficam nos handlers: `focus()`, `select()` e `dialog.close()`.
- **Edição inline:** perder o foco cancela, como o Esc. A edição passa pela mesma validação do formulário e mostra o erro na linha.
- **Busca ignora acentos** ("maca" encontra "Maçã"), em tarefas e produtos.
- **Produto esgotado:** o `Counter` mostra 0, porque `min` é 1 e `max` é o estoque 0, com os dois botões desabilitados.
- **"Adicionar" desabilita** quando todo o estoque do produto já está no carrinho.
- **O carrinho guarda nome, preço e estoque de cada item**, para aparecer no Painel e no header sem carregar o catálogo.

## 5. Pontos fracos e onde um agente erraria

- **Eventos síncronos disparados por escritas no DOM.** `blur` ao ocultar ou remover o elemento focado acontece dentro do effect. O `untrack` em `h` resolve para handlers registrados por `h`; um `addEventListener` manual não tem essa proteção (o lint barra).
- **Ler um signal fora de um binding.** `h('p', null, count())` renderiza uma vez e nunca atualiza; o certo é `h('p', null, count)` ou `() => count()`. Nada acusa esse erro: é o mais fácil de cometer e o mais difícil de ver na revisão.
- **Desestruturar props reativas.** Guardar `const v = read(value)` no corpo do componente congela o valor. Props que mudam precisam ser lidas dentro de um binding ou `computed`.
- **Região reativa refaz o DOM inteiro.** `() => cond() ? h(...) : h(...)` recria os filhos e perde foco e estado local a cada mudança de qualquer signal lido no corpo. Para listas existe `list`; para alternar visibilidade, `hidden`.
- **Estado local se perde se o elemento for reconectado.** O componente roda de novo a cada conexão. `list` evita isso com `moveBefore`, mas mover um componente à mão com `append` zera o estado em qualquer browser.
- **Effect que precisa do elemento conectado.** `render` roda antes de os nós entrarem no documento; `dialog.showModal()` falharia ali. O `Modal` contorna checando `isConnected`, então um modal que nasce aberto não abre. Falta um `onMount`.
- **Ordem de definição dos componentes.** `h('m-x')` antes do `define('m-x')` cria um elemento ainda não registrado, que só recebe as props se for definido depois. Por isso cada arquivo importa os componentes que usa e `m-app` é definido por último.
- **O lint é por regex.** Pega os casos comuns, mas não entende escopo: aceita qualquer linha com `// dom-ok` e não detecta leitura não reativa.
- **`dom.js`, `router.js` e `component.js` não têm testes no repositório.** Só a reatividade tem suíte; o resto foi coberto pelo roteiro de browser, que ficou fora do projeto.
- **Sem tipos.** Props são um objeto livre; um nome de prop errado só aparece na tela.
- **Persistência sem versão de esquema.** Mudar o formato de tarefa ou carrinho quebra quem já tem dados em `localStorage`.
