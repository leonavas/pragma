# AGENTS.md

App feito com o pragma: HTML, CSS e JS nativos, sem framework. Construa apps performáticos,
escaláveis e extensíveis. A documentação está em `core/docs/`. Leia o arquivo do assunto
antes de mexer nele.

## Regras obrigatórias

- JavaScript puro (ES modules). Nunca TypeScript, nunca JSDoc de tipos.
- Nunca HTML em JS: sem `innerHTML`, `createElement` nem template literal com markup.
  Marcação só em `.html`.
- HTML estrito: feche toda tag não void (`</p>`, `</li>`, `</option>`).
- Comportamento só por diretivas (`data-*`). Não escreva no DOM pelo JS: mude o estado.
- Valor calculado (total, rótulo, lista filtrada) é getter no `state(ctx)` ou no store. Nunca
  calcule no HTML nem guarde cópia atualizada pela action.
- Conteúdo de slot pertence a quem o escreveu: suas actions e `state` são dele, não do
  componente que o recebe.
- Estrutura de pastas fixa. Não crie pastas novas nem edite `core/`.
- Reaproveite componentes. Procure em `lib/components/` e `_components/` antes de criar.
- Overlays (modal, alert, dropdown, popover) sempre com HTML + JS, usando os componentes
  prontos. Voltar no browser ou no Android fecha o overlay, não a página.
- Modal e alert travam o scroll da página.
- Dados da página no `load` do `.server.js`, com skeleton enquanto carrega.
- CSS: utilitários de `core/docs/classes.md`; classe custom só para repetição (3+) ou caso
  refinado. Sem `style` inline, sem valores arbitrários.
- Logs com `log.*`, nunca `console.*`. Sem dado pessoal nos logs.
- Sem comentários inúteis. Comentário só para o porquê não óbvio.
- Sem dependências npm no código do app.
- Acessível: nome em todo controle, `alt` em toda imagem, label em todo campo, um `<h1>`.
  Link ativo ganha `aria-current="page"` sozinho.
- Rode `pragma check --strict` antes de terminar. Zero erros, zero avisos.

## Para fazer X, leia Y

| Para | Leia |
|---|---|
| saber onde cada arquivo vai | `core/docs/estrutura.md` |
| criar página, link, params, query | `core/docs/rotas.md` |
| ligar dados e eventos ao HTML | `core/docs/diretivas.md` |
| criar ou reaproveitar componente | `core/docs/componentes.md` |
| estado, actions, mount, stores | `core/docs/estado.md` |
| buscar dados, salvar no servidor, SSR, skeleton | `core/docs/dados.md` |
| modal, alert, dropdown, popover | `core/docs/overlays.md` |
| estilizar, tema claro/escuro | `core/docs/css.md` e `core/docs/classes.md` |
| deixar rápido, orçamentos | `core/docs/performance.md` |
| logar | `core/docs/logs.md` |
| rodar, checar, buildar, publicar, avisos `a11y/*` | `core/docs/cli.md` |

## Direcionais

- Performático: a página abre com HTML pronto e skeleton; cada página baixa só o que usa;
  navegue com `<a href>`.
- Escalável: uma forma de fazer cada coisa; estado na URL quando é filtro ou aba; estado
  compartilhado em `lib/state/`.
- Extensível: componentes pequenos com props e slots; tokens em `lib/theme.css`; nada
  duplicado entre páginas.
- Na dúvida, siga o exemplo canônico do doc. Os docs citam `exemplo/<caminho>`: o app de
  exemplo no repositório do pragma; sem acesso a ele, o exemplo do doc basta. Se o doc não
  cobre, faça o mais simples que passe no `pragma check --strict`.
