# AGENTS.md

App pragma: HTML, CSS e JS nativos, sem framework. Docs em `core/docs/`: leia o do assunto antes de mexer.

## Sintaxe

`:` liga valor (`:text`, `:href`, `:if`, `:for`+`:key`), `.` liga classe, `@` liga evento, `$` é
estado ou prop. `<c-x>` = `lib/components/x/`; `<p-x>` = `_components/x/` da pasta da página.

## Regras

- JS puro (ES modules). Sem TypeScript, JSDoc de tipos, `console.*` (use `log.*`).
- Pacote npm só se o nativo não resolve, de JS puro (ESM) e da lista de `pacotes.md`.
- Sem HTML em JS. Não escreva no DOM: mude o estado.
- HTML estrito: feche toda tag não void.
- Valor calculado é getter no estado ou no store, nunca no HTML.
- Nunca escreva comentários. O código é a documentação.
- Nomes dizem o que a coisa é ou faz. Sem vagos (`tmp`, `data`, `aux`) nem curtos (`t`, `e`),
  salvo parâmetro de arrow de uma linha e `i`, `j`, `k`, `id`, `el`, `ctx`.
- Estrutura fixa; não edite `core/`. Reaproveite componentes antes de criar.
- Overlays só com `c-modal`, `c-alert`, `c-dropdown`.
- Dados no `load` do `.server.js`, com skeleton.
- CSS: utilitários de `classes.md`; classe custom só se repete 3+ vezes. Sem `style` inline.
- Acessível: nome em todo controle, `alt`, label, um `<h1>`.
- Termine com `pragma check --strict`: zero erros, zero avisos.

## Para fazer X, leia Y

| Para | Leia |
|---|---|
| sintaxe do HTML (`:`, `.`, `@`, `$`) | `sintaxe.md` |
| onde cada arquivo vai | `estrutura.md` |
| página, link, params, query, layout | `rotas.md` |
| componente, props, slots | `componentes.md` |
| estado, actions, mount, stores, `ctx` | `estado.md` |
| load, actions de servidor, skeleton, SSR | `dados.md` |
| modal, alert, dropdown | `overlays.md` |
| estilo, tema | `css.md`, `classes.md` |
| performance | `performance.md` |
| logs | `logs.md` |
| pacote npm (instalar, quais usar) | `pacotes.md` |
| comandos, `check`, `a11y/*`, deploy | `cli.md` |

Na dúvida, siga o exemplo do doc. Se o doc não cobre, faça o mais simples que passe no `check`.
