# CLI `pragma`

| Comando | Faz |
|---|---|
| `pragma new <dir>` | cria um app com a estrutura fixa |
| `pragma dev [--port 5180]` | servidor de dev com live reload (sobe Node ≥ 22 se houver `.server.js`) |
| `pragma check [--strict]` | checa JS, HTML, CSS, orçamentos e acessibilidade |
| `pragma build` | gera `dist/` |
| `pragma serve [--port 3000]` | serve `dist/` como em produção |

Rode `pragma check --strict` antes de terminar: zero erros, zero avisos. Saída:
`arquivo:linha:coluna erro|aviso [regra] mensagem`. Corrija a causa; não contorne.

## O que o `check` barra

- JS: HTML em JS, `console.*`, TypeScript/JSDoc de tipos, import de fora do projeto, comentário
  (`js/comentario`), nome vago (`js/nome-vago`).
- HTML: tag não fechada, sintaxe desconhecida ou antiga, caminho inválido, `:for` sem `:key`,
  `:if`/`:for` fora de `<template>`, bloco sem raiz única, action inexistente, componente não
  encontrado, `style`/`on*` inline, `<script>`, comentário.
- CSS: classe desconhecida, valor arbitrário, comentário.
- Orçamentos de `pragma.json`.

## Acessibilidade (`a11y/*`, erro com `--strict`)

| Regra | Corrija com |
|---|---|
| `img-alt` | `alt` em todo `<img>`; `alt=""` se decorativa |
| `nome-acessivel` | `<button>`/`<a>` com texto, `:text`, `aria-label` ou `title` |
| `label` | campo com `<label for>`, dentro de `<label>` ou `aria-label` |
| `dialog-nome` | `<dialog>` com `aria-label` ou `aria-labelledby` |
| `clique-nao-interativo` | `@click` em `<button>`/`<a>`; senão `role` + `tabindex` |
| `heading` | sem pular nível |
| `h1-unico` | um `<h1>` por página (`:if`/`:else` contam separados) |
| `tabindex` | só `0` ou `-1` |
| `id-duplicado` | `id` único na página |
| `iframe-title` | `title` em `<iframe>` |
| `a-sem-href` | `<a>` sempre com `href`; ação é `<button>` |

`:<atributo>` conta como o atributo (`:aria-label="t.titulo"`).

## `dist/`

| Caminho | Uso |
|---|---|
| `client/` | HTML por rota e assets; sem `.server.js` vai para hospedagem estática |
| `server/worker.js` + `wrangler.jsonc` | Cloudflare Workers |
| `server/node.mjs` | `node dist/server/node.mjs` (porta `PORT`, padrão 3000) |
