# pragma

Framework agêntico e opinativo para apps web com HTML, CSS e JS nativos: páginas `.html`
pré-geradas, diretivas fechadas, bundle mínimo, sem framework de UI, e um CLI em Rust
(`pragma new|dev|build|check|serve`) com regras, docs e checks feitos para agentes.

- Contrato da v0: [SPEC.md](SPEC.md). Decisões e motivos: [DECISOES.md](DECISOES.md).

## Pastas do repositório

```
pragma/
  core/
    SPEC.md  DECISOES.md  EQUIPE.md  README.md
    docs/        docs para LLMs; copiadas para <app>/core/docs/
    runtime/     JS do framework; copiado para <app>/core/
      server/    handler fetch, SSR, adaptador Node, logs do servidor
    starter/     esqueleto de app usado por `pragma new` (AGENTS.md, routes/, lib/…)
    cli/         workspace Rust
      crates/pragma-diag      diagnósticos compartilhados
      crates/pragma-css       subconjunto do Tailwind v4 + base CSS
      crates/pragma-jslint    regras de JS (oxc)
      crates/pragma-compiler  build, análise, diretivas
      crates/pragma-cli       binário `pragma`
    exemplo/     app de exemplo gerado com `pragma new`
  tests_1/       experimento: SvelteKit × astra × fable (ver COMPARACAO.md)
```
