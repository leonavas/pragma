# Performance

Já é rápido por padrão: HTML pronto, fetch do `load` no `<head>`, JS só do que a página usa,
prefetch de links, assets com hash, CSS só com as classes usadas, orçamentos no `check`.

- Navegue com `<a href>`.
- Skeleton no formato do conteúdo final.
- `<img>` com `width` e `height`; `loading="lazy"` abaixo da dobra; AVIF/WebP em `public/`.
- `:for` com `:key` estável.
- Página sem interação não tem `.js`.
- Código pesado de uma página fica nela; se só roda após interação, `await import()` na action.
- `load` devolve só o que a página mostra. Chamadas independentes: `Promise.all`.
- Não aumente orçamento em `pragma.json`: reduza o código.
- Não importe módulo grande em `lib/state/`, não busque dados no `mount`, não ligue `ssr` por padrão.
