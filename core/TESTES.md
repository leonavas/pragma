# Testes do pragma

Rode a partir de `core/cli/`, com `cargo xtask test [alvo]`:

| Alvo | O que roda |
|---|---|
| *(nenhum)* | Rust (`cargo test`) + runtime do servidor |
| `rust` | crates: css, jslint, compiler (inclui a11y), cli |
| `servidor` | handler, SSR, logs OTel, adaptador Node |
| `cliente` | diretivas, router, loader, stores (Chromium) |
| `overlays` | modal, alert, dropdown, voltar, scroll lock (Chromium e Firefox) |
| `e2e` | app de exemplo inteiro em `pragma serve` e `pragma dev` |
| `tudo` | todos acima |

- Implementado em `core/cli/xtask/` (padrão `cargo xtask`; alias em `core/cli/.cargo/config.toml`).
- Na 1ª vez que um teste de browser roda, instala o Playwright em `~/.cache/pragma-tools`
  (fora do repo) e os browsers em `~/.cache/ms-playwright`. `PRAGMA_TOOLS_DIR` troca a pasta.
- O `e2e` compila o `pragma` em `core/cli/target/release/pragma` e usa esse binário.

Testar o framework na mão:

```sh
cd core/exemplo
pragma dev            # http://127.0.0.1:5180, live reload
pragma check --strict # erros + avisos de acessibilidade
pragma build && pragma serve
```
