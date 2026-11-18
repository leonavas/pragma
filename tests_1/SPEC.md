# tests_1: a mesma aplicação, três implementações

Objetivo: comparar três formas de construir o mesmo app web e medir o tamanho do
bundle, a clareza do código e a facilidade de manutenção por agentes.

| Pasta | Abordagem |
|---|---|
| `sveltekit/` | Svelte 5 (runes) + SvelteKit 2, usado como referência de mercado |
| `astra/` | HTML/CSS/JS nativos, **sem signals**: estado central + `dispatch`, componentes com `mount`/`update`/`destroy` |
| `fable/` | HTML/CSS/JS nativos, **com signals**: `signal`/`computed`/`effect`/`batch` + escopo de descarte, custom elements |

Todas implementam **exatamente** esta especificação. Desvios devem ser
justificados no `RESULTADO.md` da pasta.

## O app: "Mercadinho"

Uma SPA com 3 páginas, layout comum e estado compartilhado entre páginas.

### Layout (presente em todas as páginas; não conta nos 6 por página)

- **AppHeader**: nome do app e navegação com `NavLink` para as 3 páginas. O link
  da rota atual fica destacado (`aria-current="page"`).
- **CartBadge** (no header): quantidade total de itens no carrinho. Atualiza na
  hora, em qualquer página.
- Rota desconhecida: página 404 simples, com link para o início.
- O `<title>` muda a cada rota.

### Página 1: `/` Painel

| # | Componente | O que faz | Conceito |
|---|---|---|---|
| 1 | `Counter` | Usado **2 vezes** com props diferentes: `{label:"Cliques", initial:0, step:1}` e `{label:"Pulos de 5", initial:10, step:5, min:0, max:50}`. Botões − e + desabilitam nos limites. | estado local, props, reuso |
| 2 | `StatCard` | Usado **3 vezes**: "Tarefas pendentes", "Itens no carrinho" e "Total do carrinho" (em R$). Props: `title`, `value`, `hint?`, `tone` (`neutro`/`ok`/`alerta`). Fica `alerta` quando há 0 pendentes ou o carrinho está vazio. | componente de apresentação, estado global derivado |
| 3 | `Clock` | Mostra a hora (HH:MM:SS), atualizando a cada segundo. O timer **para** ao sair da página. | efeito com limpeza, ciclo de vida |
| 4 | `ThemeToggle` | Alterna claro/escuro. Aplica `data-theme` no `<html>` e persiste em `localStorage`. Sobrevive a refresh. | estado global persistido, efeito no DOM externo |
| 5 | `Tabs` | 3 abas ("Sobre", "Atalhos", "Contador"). O conteúdo de cada aba é passado de fora (composição). A aba "Contador" contém **outro `Counter`** com `{label:"Na aba", initial:3, step:3}`. Teclas ← → trocam de aba (padrão ARIA tabs). | composição/children, acessibilidade |
| 6 | `Modal` | Botão "Sobre o app" abre um `<dialog>` com título e conteúdo passados de fora. Fecha com Esc, botão Fechar ou clique no fundo. O foco volta ao botão que abriu. | composição, evento filho→pai |

### Página 2: `/tarefas` Tarefas

| # | Componente | O que faz | Conceito |
|---|---|---|---|
| 1 | `SearchInput` | Busca por texto no título das tarefas, com placeholder "Buscar tarefa…". Filtra enquanto digita. | input controlado, reuso (também na loja) |
| 2 | `FilterBar` | Opções: Todas / Pendentes / Concluídas. O valor vive na **URL** (`?filtro=pendentes`), sobrevive a refresh e funciona com voltar/avançar. | estado na URL, reuso (também na loja) |
| 3 | `TodoForm` | Campo + botão Adicionar. Validação: obrigatório, 3 a 80 caracteres, sem duplicata (ignorando maiúsculas). Erro exibido abaixo do campo. Enter envia; o campo limpa e volta o foco após o sucesso. | formulário, validação |
| 4 | `TodoList` | Lista com **chave** por id. Cada item: checkbox de concluir, título, botão remover e **edição inline** com duplo clique (Enter salva, Esc cancela). Mostra "Nenhuma tarefa" quando o filtro esvazia a lista. Persiste em `localStorage`. | lista com chave, estado global persistido |
| 5 | `StatCard` | Reuso: "Concluídas" com valor "X de Y" e tom `ok` quando X = Y > 0. | reuso com outras props |
| 6 | `Modal` | Reuso: o botão "Limpar concluídas" abre uma confirmação. Cancelar não faz nada; Confirmar remove as concluídas. | reuso com outro conteúdo |

Estado inicial (primeira visita, sem `localStorage`): 3 tarefas, sendo
"Comprar café" (pendente), "Pagar contas" (concluída) e "Ligar para a farmácia"
(pendente).

### Página 3: `/loja` Loja

| # | Componente | O que faz | Conceito |
|---|---|---|---|
| 1 | `SearchInput` | Reuso: placeholder "Buscar produto…". Filtra por nome. | reuso com outras props |
| 2 | `FilterBar` | Reuso: Todas / Mercearia / Limpeza / Hortifruti. O valor fica na URL (`?categoria=limpeza`). | reuso com outras opções |
| 3 | `ProductGrid` | Carrega `/produtos.json` (cópia de `shared/produtos.json`) com **atraso simulado de 400 ms**. Estados: carregando (esqueleto), erro (mensagem + "Tentar de novo") e vazio ("Nenhum produto"). `?falhar=1` na URL força o erro, para testar. Os dados ficam em cache: voltar à loja não recarrega. | dados assíncronos, estados de carga |
| 4 | `ProductCard` | Um por produto, dentro do grid: nome, categoria, preço em R$ e selo "Esgotado" quando o estoque é 0. Botão "Adicionar" fica desabilitado se esgotado. | reuso em lista |
| 5 | `Counter` | Reuso **dentro do `ProductCard`** como seletor de quantidade: `{label:"Qtd", initial:1, step:1, min:1, max:estoque}`. "Adicionar" usa essa quantidade. | reuso aninhado, props dinâmicas |
| 6 | `CartSummary` | Lista os itens do carrinho (nome, qtd, subtotal), com botões −/+ e remover por item, total geral e botão "Esvaziar". A quantidade nunca passa do estoque. | estado global, estado derivado |

O carrinho é global: persiste entre páginas e em `localStorage`. O `CartBadge`
e os `StatCard`s do Painel refletem o carrinho na hora.

### Rota dinâmica (subrota da Loja, não conta como página)

`/loja/:id`: detalhe do produto. Reusa `ProductCard`, ou parte dele, e
`Counter`, com link "← Voltar para a loja". Um id inexistente mostra "Produto
não encontrado". Clicar no nome do produto no grid leva a essa rota.

## Requisitos comuns

- **CSS**: use `shared/preflight.css` (o reset do Tailwind v4, adaptado para CSS
  puro) como primeira camada. Não use Tailwind nem outra lib de CSS. Organize em
  `@layer reset, tokens, base, layout, components, utilities`. Tokens (cores,
  espaços, raio, fonte) vão em custom properties, com tema claro e escuro via
  `[data-theme="dark"]`. O visual deve ser simples, limpo e consistente entre as
  três versões; não precisa ser idêntico.
- **Rotas**: navegação client-side sem recarregar a página. Voltar/avançar do
  browser funcionam. Deep link funciona (abrir `/tarefas?filtro=concluidas`
  direto, ou dar refresh em `/loja/cafe`) **no dev e no preview do build**.
- **Acessibilidade básica**: HTML semântico, labels em inputs, foco visível,
  `aria-current` no link ativo.
- **Formato de dinheiro**: `Intl.NumberFormat('pt-BR', {style:'currency', currency:'BRL'})`.
- **Sem dependências de runtime** em `astra/` e `fable/`. Dependência de
  desenvolvimento permitida: **apenas `esbuild`**, para bundle/minificação e
  servidor de dev. Todo o código de "framework" (reatividade, componentes,
  router) é escrito na própria pasta.
- **JS**: módulos ES; tipos via JSDoc são opcionais.

## Critérios de aceite (cada implementação verifica todos)

1. As 3 páginas, a 404 e `/loja/:id` renderizam, e a navegação pelos links não recarrega a página.
2. Voltar/avançar e deep link funcionam, incluindo os filtros na URL.
3. Os `Counter`s respeitam `min`/`max` e as 4 instâncias são independentes. As 3 do Painel/Tabs e as da loja não compartilham estado.
4. Adicionar ao carrinho na Loja atualiza o `CartBadge` na hora. Ao ir ao Painel, os `StatCard`s mostram os valores certos.
5. Tarefas: adicionar, validar, concluir, editar inline, remover, buscar, filtrar e limpar concluídas com confirmação.
6. Refresh mantém tarefas, carrinho e tema.
7. O `Clock` para ao sair do Painel: nenhum timer ou listener vazando. Verificar, por exemplo, contando os ticks ou com um log temporário.
8. A loja mostra o esqueleto durante a carga e trata o erro com `?falhar=1`. Voltar à loja não refaz o fetch.
9. O modal fecha com Esc e devolve o foco. As Tabs funcionam com o teclado.
10. O build de produção gera arquivos estáticos que funcionam servidos com fallback para `index.html`.

## Entregável: `RESULTADO.md` em cada pasta

1. Como rodar: instalar, dev, build e preview (comandos exatos).
2. **Medidas** (do build de produção):
   - JS total, gzip (`gzip -9 -c arquivo | wc -c`, somando todos os `.js`);
   - JS carregado na primeira visita a `/`, gzip;
   - CSS total, gzip;
   - linhas de código do app por tipo (`.js`/`.svelte`/`.css`/`.html`), excluindo `preflight.css`, `node_modules` e saída de build;
   - linhas do "runtime" próprio (reatividade + componentes + router), quando houver.
3. Checklist dos 10 critérios de aceite, cada um com ✅/❌ e como foi verificado.
4. Decisões de arquitetura e onde cada conceito vive (arquivo:linha): estado, reatividade, componente, rota, efeito com limpeza, lista com chave.
5. Pontos fracos e o que um agente erraria com mais facilidade nesta abordagem.
