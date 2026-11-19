# Astra — DX da nova API

Escopo: proposta, docs syntax/components/state/data, starter, AGENTS, exemplos e navegador em 390/1440 px. Somente leitura no projeto, exceto este relatório.
Resumo: `let` + funções + serviços importados reduz o código; `fn()` em JS e `{$fn}` em HTML está bem explicado. Sem P0. Dois P1 e três P2 abaixo.

## Achados

**P1 — `async mount()` é aceito, mas perde o cleanup.** `core/runtime/page.js:34`, `core/runtime/page.js:79`, `core/docs/state.md:112`.
A doc manda mover `await` para `mount`; o runtime guarda a Promise como `o.off` e tenta chamá-la na saída. Reproduzido no scratch: checker limpo, depois `TypeError: o.off is not a function`; o timer fica sem limpeza.
Correção: aceitar Promise de cleanup, tratar rejeição e executar a limpeza se o componente já saiu quando ela resolver; ou rejeitar mount assíncrono no checker e documentar uma alternativa síncrona.

**P1 — O AGENTS do próprio site ensina a API removida.** `site/AGENTS.md:7`, `site/AGENTS.md:17`, `site/AGENTS.md:36`.
Ainda recomenda `:text`, getters e `ctx`; faltam `<script>`, `export let` e `shared`. Um agente que segue esse arquivo produz código que o checker rejeita.
Correção: migrar essas instruções a partir de `core/starter/AGENTS.md:7`; manter apenas regras locais que ainda valem.

**P2 — Leitura antecipada de `data` passa no checker e falha no boot.** `core/docs/data.md:81`, `core/runtime/page.js:61`, `core/cli/crates/pragma-jslint/src/script.rs:461`.
`import { data } from 'pragma'; const total = data.items.length` passou sozinho em `check --strict`; o navegador registrou três TypeErrors antes do load terminar. O componente recuperou após o load.
Correção: detectar leitura direta de `data` durante a inicialização e indicar função derivada + guarda de loading; refletir a mesma restrição no resumo do AGENTS.

**P2 — “Props read-only” omite que objetos continuam mutáveis por referência.** `core/docs/components.md:40`, `site/lib/components/source-tab/source-tab.html:8`, `site/lib/components/source-expand/source-expand.html:9`.
Os exemplos alteram objetos recebidos em `export let files`; `js/prop-write` só impede reatribuir o binding (`core/cli/crates/pragma-jslint/src/script.rs:439`). `let local = prop` também preserva a referência.
Correção: dizer explicitamente “binding somente leitura, objeto compartilhado mutável”, explicar cópia local e mostrar um exemplo; se a intenção for imutabilidade profunda, alinhar runtime e exemplos ao contrato.

**P2 — O erro de prop sugere perpetuar um typo.** `core/cli/crates/pragma-compiler/src/expand.rs:406`.
Para `lable="Example"` quando existe `label`, o erro lista `label`, mas recomenda declarar `export let lable = …`. Um agente pode seguir a correção e criar uma prop sem uso.
Correção: sugerir `label` quando há correspondência próxima; recomendar nova declaração apenas como alternativa.

## Oito componentes errados

Scratch: `/tmp/pragma-astra-dx-MWptvV/app`, criado com `pragma new`; executado `pragma check --strict`. Sete casos detectados, oito erros totais e zero warnings.
| Caso | Resultado e qualidade |
|---|---|
| `:text="$name"` | `html/text-removed`, linha/coluna e substituição exata por `{$name}`. Bom. |
| `{$total()}` | `html/invalid-path`, sugere `$total`. Bom, embora a função já exista. |
| Incrementar `export let count` | `js/prop-write`, aponta a escrita e sugere estado local. Bom para escalares. |
| `const items = []` + `items.push()` | `js/const-state`, aponta a declaração e recomenda `let`. Bom. |
| `@click="increase($count + 1)"` | `html/invalid-path`, explica argumentos válidos e proíbe expressões. Bom. |
| Importar diretamente `lib/state/cart.js` | `js/store-import`, indica `shared`; há um segundo `html/unknown-name` para o handler importado. |
| Prop `lable` em vez de `label` | `html/unknown-prop`, localização correta, sugestão ruim descrita acima. |
| `const total = data.items.length` | Nenhum diagnóstico; confirmado o erro no navegador. |

Após corrigir os sete casos detectados, o checker aprovou o oitavo. Uma variação posterior com `async mount` também passou e reproduziu o erro de cleanup na navegação. Não rodei build/check do projeto.

## Exemplos, contrato e visual

`site/routes/_components/task-list/task-list.html:2` e `core/example/lib/state/tasks.js:1` mostram bem estado e ações. `core/starter/routes/index.html:2` usa serviços e estado sem a API antiga. As ocorrências restantes de `actions = {` em `.server.js` são válidas; snippets rejeitados e notas de migração são intencionais.
`core/docs/syntax.md:32` explica a exceção de braces pelo pai imediato de `<pre>/<code>`; é uma regra incomum, mas explícita. Os docs também distinguem funções derivadas, argumentos de eventos, estados por instância e props sem script. O contrato de objetos e o lifecycle assíncrono precisam da precisão indicada acima.
Visual: landing e Syntax em 390×844; State em 1440×900. Sem overflow horizontal ou nova quebra visual observada nessa amostra. Filtro de tarefas, interpolação escapada (`<b>bold</b>`) e troca de abas de fonte funcionaram; sem erros de console nessas interações do site.

## Top 5

1. P1 — Corrigir cleanup de `async mount()` ou rejeitá-lo com orientação válida.
2. P1 — Migrar `site/AGENTS.md` para a API atual.
3. P2 — Diagnosticar acesso antecipado a `data`.
4. P2 — Definir a mutabilidade de props de objetos e a cópia de estado local.
5. P2 — Sugerir a prop existente quando o nome contém typo.
