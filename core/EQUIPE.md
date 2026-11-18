# Equipe da v0 — regras para os agentes

Contrato: `core/SPEC.md` (leia inteiro antes de começar; é a fonte de verdade).
Contexto: `core/DECISOES.md` (pode estar sendo reescrito agora; se divergir, vale o SPEC).

## Donos (mexa SÓ no que é seu; os outros estão editando o resto agora)

| Papel | Arquivos |
|---|---|
| css | `core/cli/crates/pragma-css/**`, `core/docs/classes.md` |
| jslint | `core/cli/crates/pragma-jslint/**` |
| compiler | `core/cli/crates/pragma-compiler/**` |
| cli | `core/cli/crates/pragma-cli/**` |
| runtime-cliente | `core/runtime/{boot,router,directives,state,page,loader}.js`, `core/runtime/test/client/**` |
| runtime-servidor | `core/runtime/log.js`, `core/runtime/server/**`, `core/runtime/test/server/**` |
| overlays | `core/runtime/overlay.js`, `core/runtime/test/overlay/**`, `core/starter/**` (menos AGENTS.md e CLAUDE.md), `core/docs/overlays.md` |
| docs | `core/DECISOES.md`, `core/README.md`, `core/docs/**` (menos classes.md e overlays.md), `core/starter/AGENTS.md`, `core/starter/CLAUDE.md` |

Ninguém edita `SPEC.md`, `EQUIPE.md`, `pragma-diag` nem `core/cli/Cargo.toml` da raiz; peça ao
coordenador. Você pode adicionar dependências no `Cargo.toml` do seu crate.

## Regras

- Precisa de algo de outro dono? Implemente contra o contrato e cite na resposta final.
  Não mude assinaturas públicas existentes; pode acrescentar funções.
- Rust: `CARGO_TARGET_DIR=/tmp/pragma-target-<papel>` (evita travar o build dos outros).
  Edition 2024, poucas dependências, versões atuais.
- JS: puro, ES modules, sem TypeScript, sem JSDoc de tipos, sem dependências de runtime.
  Ferramentas de teste (ex.: Playwright) só instaladas em `/tmp/pragma-tools-<papel>`, nunca no repo.
- Sem comentários inúteis: comentário só para o porquê não óbvio. Código pequeno e direto,
  sem abstração especulativa.
- Mensagens ao usuário (erros, CLI, docs) em português. Identificadores do framework em inglês.
- Testes obrigatórios para o que você entrega; rode-os antes de terminar.
- Resposta final: no máximo 15 linhas — o que entregou, como testar, o que faltou e onde
  interpretou ou divergiu do SPEC.
