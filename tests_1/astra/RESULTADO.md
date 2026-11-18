# Resultado — Mercadinho / Astra

Implementação concluída em HTML, CSS e módulos JavaScript nativos, sem signals e sem dependências de runtime. Build de produção executado com sucesso. Os dez cenários de aceite passaram no Chromium headless; deep links também foram executados no modo dev. Os servidores iniciados para a verificação foram encerrados.

## 1. Como rodar

Requisitos: Node.js e npm. Ambiente usado: Node 26.12.0, npm 12.2.1, esbuild 0.27.7 (fixado pelo lockfile).

```bash
cd /home/leonavas/projects/personal/pragma/tests_1/astra
npm ci
npm run dev
```

Dev: `http://127.0.0.1:5172`. Encerre com Ctrl+C antes de iniciar outro servidor.

```bash
npm run build
npm run preview
```

Preview: `http://127.0.0.1:5172`. Ambos usam esbuild e fallback para `dist/index.html`. O preview serve os arquivos já produzidos; não recompila a aplicação. A pasta `dist/` pode ser publicada em hospedagem estática configurada com fallback para `index.html`.

### Reproduzir a verificação

O Playwright não está no `package.json` nem no lockfile do projeto. Foi instalado em diretório temporário, conforme autorizado:

```bash
npm install --prefix /tmp/pragma-astra-browser playwright
```

Com o preview ativo em outro terminal, usando Chromium disponível no sistema:

```bash
node scripts/acceptance.mjs
```

A execução final usou o Chromium headless isolado já presente na máquina:

```bash
CHROMIUM_PATH=/home/leonavas/.cache/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-linux64/chrome-headless-shell node scripts/acceptance.mjs
```

É possível apontar `CHROMIUM_PATH` para outro Chromium instalado e `PLAYWRIGHT_MODULE` para o `index.mjs` de outra instalação temporária do Playwright. Depois de encerrar o preview, inicie `npm run dev` e execute, com as mesmas variáveis se necessário:

```bash
node scripts/deep-links.mjs
```

Encerre o dev antes de deixar o build final de produção:

```bash
npm run build
node scripts/measure.mjs
```

## 2. Medidas do build de produção

| Medida | Resultado |
| --- | ---: |
| JS total, gzip nível 9 | **8.188 bytes** |
| JS carregado na primeira visita a `/`, gzip nível 9 | **8.188 bytes** |
| CSS total, gzip nível 9, incluindo reset | **3.747 bytes** |
| JS total minificado, sem gzip | 23.666 bytes |
| CSS total minificado, sem gzip | 12.497 bytes |
| Linhas `.js` do código entregue ao navegador, incluindo runtime próprio | **769** |
| Linhas `.svelte` | **0** |
| Linhas `.css`, excluindo `preflight.css` | **156** |
| Linhas `.html` | **16** |
| Linhas do runtime próprio em `src/lib/` | **111** |
| Linhas `.js` específicas do app, descontando runtime | **658** |

O único bundle JavaScript é `dist/assets/app.js`, carregado integralmente na primeira visita. Não há imports dinâmicos, fontes externas, ícones de bibliotecas nem scripts de terceiros. CSS está em `dist/assets/app.css`.

Medições gzip usam os comandos exigidos, sem substituir por uma estimativa:

```bash
gzip -9 -c dist/assets/app.js | wc -c
gzip -9 -c dist/assets/app.css | wc -c
```

A contagem é de linhas físicas, incluindo comentários e linhas em branco, não de statements. CSS agrupa declarações por regra. Entram `src/**/*.js`, `src/styles.css` e `index.html`. Não entram `src/preflight.css`, `node_modules`, `dist`, dados JSON, evidências ou documentação. Os scripts Node `.mjs` são ferramentas, não código do app: 22 linhas de build, 300 de aceite, 28 de deep links e 26 de medição (376 no total).

Runtime: `component.js` 27 linhas; `store.js` 26; `router.js` 25; `list.js` 33. A soma está incluída nas 769 linhas de JS, não é uma parcela adicional.

Dados reproduzíveis: [evidence/measures.json](evidence/measures.json). As cópias do reset e de `produtos.json` foram comparadas byte a byte com os originais usando `cmp`.

## 3. Critérios de aceite

Evidência automatizada: [scripts/acceptance.mjs](scripts/acceptance.mjs), [evidence/acceptance.json](evidence/acceptance.json), [scripts/deep-links.mjs](scripts/deep-links.mjs) e [evidence/dev-deep-links.json](evidence/dev-deep-links.json).

| # | Resultado | Verificação executada |
| --- | --- | --- |
| 1 | ✅ | Painel, Tarefas, Loja, detalhe `cafe`, produto inexistente e 404 renderizados. Títulos e `aria-current` verificados. Um marcador em `window` permaneceu intacto ao navegar pelos links, comprovando ausência de recarga. |
| 2 | ✅ | Deep links e refresh em `/tarefas?filtro=concluidas` e `/loja/cafe` no preview e no dev. Histórico de filtros de tarefas e categorias da loja testado com voltar/avançar; refresh conserva o filtro da URL. |
| 3 | ✅ | Cliques, Pulos de 5 e Na aba independentes; Pulos de 5 chega a 0 e 50 com botões desabilitados. Café muda para 2 enquanto arroz permanece em 1; alface para em 3. Produto esgotado não permite adicionar. Ver ressalva de intervalo impossível abaixo. |
| 4 | ✅ | Adicionar duas unidades de café atualiza o badge para 2 e o total para R$ 65,80. Adições repetidas de alface não ultrapassam estoque 3. Decremento e remoção refletem no badge. Ao navegar ao Painel, StatCards mostram 2 e R$ 65,80. |
| 5 | ✅ | Obrigatório, mínimo, limite nativo de 80 e rejeição de valor programático de 81 caracteres; duplicata sem distinguir maiúsculas. Adição por Enter limpa e foca o campo. Concluir, remover, buscar, filtrar, duplo clique, Enter salva, Esc cancela, confirmar/cancelar limpeza. Renomear uma tarefa para fora da busca produz o estado vazio. A edição conserva o mesmo nó, foco e seleção quando outra tarefa é atualizada. |
| 6 | ✅ | Após refresh, tema escuro e carrinho de duas unidades permanecem; tarefa criada permanece e tarefa removida continua ausente. Painel calcula o total usando o carrinho restaurado, antes de carregar o catálogo. |
| 7 | ✅ | Instrumentados `setInterval`, `clearInterval` e listeners associados a `AbortSignal`. Cinco ciclos Painel→Tarefas mantêm o número de listeners ativos estável; existe um intervalo no Painel e zero fora dele. Após sair, o contador de ticks fica inalterado por 1,1 segundo. |
| 8 | ✅ | Esqueleto observado durante o atraso de 400 ms. `?falhar=1` gera erro; Tentar de novo recupera o catálogo. Falha HTTP 503 também foi injetada e recuperada. Busca sem resultado mostra Nenhum produto. Contagem de requests comprova que voltar à loja não faz novo fetch. |
| 9 | ✅ | Modal abre e fecha com Esc, Fechar e clique no fundo; foco retorna ao disparador nos três caminhos. Tabs alternam com setas, inclusive retorno circular, com `aria-selected` atualizado. |
| 10 | ✅ | Build minificado concluído. Preview serviu HTML por fallback em rota profunda, JS, CSS e JSON com HTTP 200. A suíte completa rodou nesse build. Painel, Tarefas, Loja e detalhe não apresentam overflow horizontal a 390 px; nenhum erro JavaScript foi capturado. |

Capturas conferidas: [desktop](evidence/desktop.png), [loja mobile em tema escuro](evidence/mobile.png), [tarefas em tema escuro](evidence/dark-tasks.png). O Painel também foi inspecionado no navegador visível do Sheltie. O detector mecânico de CSS não apontou ocorrências nos fontes inspecionados; avisou que a URL absoluta do CSS do HTML é resolvida apenas no build servido.

A verificação de ciclo de vida é instrumental e comportamental, não um perfil completo de heap. Não foram executados Firefox, Safari ou leitor de tela; esses testes não são alegados como aprovados.

## 4. Arquitetura e localização dos conceitos

| Conceito | Localização | Decisão |
| --- | --- | --- |
| Estado canônico e ações | `src/state.js:20`, `src/state.js:49`; `src/lib/store.js:1` | Tarefas, carrinho, tema e catálogo no store. Reducer puro; todas as mudanças compartilhadas passam por `dispatch`. Fila evita notificações recursivas intercaladas. |
| Derivações puras | `src/state.js:56` | Quantidade, total em centavos e tarefas pendentes calculados a partir do estado. Sem cópias de contadores globais para sincronizar. Dinheiro usa `Intl.NumberFormat` pt-BR/BRL. |
| Atualização da UI | `src/app.js:28`, `src/app.js:45` | Uma assinatura encaminha o novo estado ao header e à página atual. Componentes atualizam os nós existentes; texto e valor de input só são atribuídos quando mudam. Sem signals, dependências automáticas ou virtual DOM. |
| Contrato de componente | `src/lib/component.js:2`; exemplo `src/components/common.js:4` | `mount(root, props)` retorna `update(props)` e `destroy()`. Template HTML constante apenas na montagem. Dados de usuário entram como texto/valor. Filhos e limpezas têm propriedade explícita via `own` e `cleanup`. |
| Rotas | `src/lib/router.js:1`, `src/app.js:18`, `src/app.js:44` | History API, interceptação de links internos e `popstate`. Troca de pathname destrói a página e monta a próxima. Query muda props sem desmontar a página, preservando busca e foco. |
| Filtros na URL | `src/lib/router.js:20`, `src/pages/tasks.js:11`, `src/pages/shop.js:11` | `URLSearchParams`; cliques criam entradas de histórico. Valores desconhecidos são tratados como Todas. |
| Efeito com limpeza | `src/components/common.js:46`; `src/lib/component.js:5` | Relógio libera o intervalo no destroy. Cada componente tem `AbortController`; destroy aborta seus listeners. Timer da falha simulada também é liberado. |
| Efeitos globais | `src/state.js:61` | Aplicação do tema no `<html>` e persistência. Escritas no armazenamento ocorrem apenas quando tarefas, carrinho ou tema mudam. |
| Lista com chave | `src/lib/list.js:2`; `src/components/todos.js:78` | `Map` por id; itens existentes recebem update. Itens removidos são destruídos, e inserção/movimentação usa os nós existentes. Tarefas, produtos e carrinho compartilham esse helper. |
| Estado local | `src/components/common.js:4`, `src/components/todos.js:24` | Quantidade de cada Counter, aba ativa, rascunhos e edição são locais às instâncias. Não se misturam com o estado persistido. |
| Composição | `src/components/common.js:109`, `src/components/common.js:151` | Modal e Tabs recebem funções que montam conteúdo e retornam o mesmo contrato. O Counter da aba é montado pelo Painel. |
| Eventos filho→pai | `src/components/common.js:109` | Modal recebe `onConfirm`/`onClose`. Confirmação de limpeza dispara a ação definida na página de Tarefas. |
| Catálogo e cache | `src/state.js:69`, `src/components/shop.js:31` | Uma promise compartilhada evita cargas concorrentes. Dados ficam no store entre rotas; refresh inicia outra sessão de cache. A carga pertence à aplicação e pode terminar depois de sair da loja; não mantém listeners de páginas destruídas. |
| CSS | `src/styles.css:1` | Ordem `reset, tokens, base, layout, components, utilities`. Reset copiado e importado na primeira camada. Tokens de cor, espaços, raios e fonte; claro/escuro por `data-theme`. |

## 5. Pontos fracos e erros prováveis para um agente

- A atualização é explícita: um agente pode acrescentar uma prop ou derivação e esquecer de encaminhá-la no `update` da página. Não há descoberta automática de dependências.
- Toda ação global percorre os componentes montados da página. Para nove produtos isso é simples e barato; listas grandes exigiriam seletores, memoização ou atualização seletiva. Não há batching automático entre ações separadas.
- Ciclo de vida ainda exige disciplina: novos filhos devem ser registrados em `own`, efeitos em `cleanup`, listeners em `on`. Criar um listener diretamente sem sinal escaparia desse contrato.
- Eventos podem remover o próprio item: a edição é encerrada antes de despachar o novo título, porque uma mudança pode tirar a tarefa do filtro. Inverter essa ordem provocaria acesso a nós já desmontados.
- O helper de chave preserva identidade, mas o agente precisa fornecer ids únicos e estáveis. Itens realmente filtrados para fora da lista são destruídos; ao voltar, seu estado local reinicia. Estado de negócio continua no store.
- Não há sincronização entre abas nem revalidação de catálogo durante a sessão. O carrinho salva um snapshot do nome, preço e estoque para que o Painel funcione após refresh sem buscar produtos. Um catálogo mutável exigiria reconciliação desses snapshots.
- Se `localStorage` estiver indisponível, a aplicação continua funcionando em memória, sem persistência. JSON inválido recupera os valores iniciais.
- O bundle inclui as três páginas desde a primeira visita. Isso evita runtime adicional de carregamento, mas não minimiza o JS inicial por rota; é uma escolha explícita deste experimento.

## 6. Ajustes e desvios da especificação

- **Intervalo impossível no esgotado:** a spec combina `initial: 1`, `min: 1` e `max: estoque`, inclusive quando estoque é 0. Não existe valor que satisfaça esse intervalo. Nesse caso o Counter normaliza o limite superior para 1 e fica totalmente desabilitado; Adicionar também fica desabilitado. Nenhum item esgotado entra no carrinho. Para todo estoque positivo, o limite superior é o estoque real.
- **Acessibilidade da edição:** além do duplo clique exigido, há um botão Editar operável pelo teclado.
- **Retry de falha simulada:** Tentar de novo remove `falhar=1` com `replaceState` e recupera o catálogo; a URL deixa de ordenar a falha. Não cria entrada adicional de histórico.
- Não há omissões funcionais conhecidas, frameworks, signals, dependências de runtime ou dependências de desenvolvimento além do esbuild. Nenhum original compartilhado nem pasta irmã foi alterado.
