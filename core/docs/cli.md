# CLI `pragma`

| Comando | Faz |
|---|---|
| `pragma new <dir>` | cria um app com a estrutura fixa, `core/` e `core/docs/` |
| `pragma dev [--port 5180] [--host 127.0.0.1]` | servidor de dev com live reload |
| `pragma check [--strict]` | checa JS, HTML, CSS, orçamentos e acessibilidade |
| `pragma build` | gera `dist/` para produção |
| `pragma serve [--port 3000] [--host 127.0.0.1]` | serve `dist/` como em produção, sem watch |

## Fluxo

1. `pragma dev` enquanto trabalha. CSS troca sem recarregar; o resto recarrega.
2. `pragma check --strict` antes de terminar qualquer tarefa. Zero erros, zero avisos.
3. `pragma build` para publicar.

Com `*.server.js`, o `pragma dev` sobe Node (≥ 22) para `load`, actions, `/_otel` e
páginas SSR. Os logs de cliente e servidor aparecem legíveis no terminal. `pragma dev` e
`pragma serve` encerram o Node junto (Ctrl+C ou SIGTERM).

## O que o `check` barra

- JS: HTML em JS (`innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`,
  `createElement`, template literal com markup); `console.*`; TypeScript ou JSDoc de tipos;
  import fora do projeto e de `core/`.
- HTML: tag não fechada ou sem abertura (`html/sintaxe`); diretiva desconhecida; raiz de
  caminho inválida; `data-for` sem `data-key`; `if`/`for` fora de `<template>`;
  `data-else`/`data-empty` fora de lugar; bloco sem raiz única; action inexistente no dono
  léxico; componente não encontrado; `style` ou `on*` inline; `<script>` em `routes/` ou `lib/`.
- CSS: classe desconhecida; valor arbitrário.
- Orçamentos de `pragma.json`.

Saída: `arquivo:linha:coluna erro|aviso [regra] mensagem`. Corrija a causa; não contorne a regra.

## Avisos de acessibilidade (`a11y/*`)

Sobre a página expandida (layout + página + componentes), apontando o arquivo de origem. Não
bloqueiam o `check`; com `--strict` viram erro.

| Regra | Corrija com |
|---|---|
| `img-alt` | `alt` em todo `<img>`; `alt=""` se decorativa |
| `nome-acessivel` | `<button>`/`<a>` com texto, `data-text`, `aria-label`, `aria-labelledby` ou `title` |
| `label` | input, select, textarea com `<label for>`, dentro de `<label>` ou `aria-label` (isentos: `type` hidden, submit, reset, button, image) |
| `dialog-nome` | `<dialog>` com `aria-label` ou `aria-labelledby` |
| `clique-nao-interativo` | `data-on-click` em `<button>`/`<a>`/controle; senão `role` + `tabindex` |
| `heading` | sem pular nível (`h2` → `h4`) |
| `h1-unico` | um `<h1>` por página (ramos `data-if`/`data-else` contam separados) |
| `tabindex` | `tabindex` só `0` ou `-1` |
| `id-duplicado` | `id` único na página expandida |
| `iframe-title` | `title` em todo `<iframe>` |
| `a-sem-href` | `<a>` sempre com `href`; ação é `<button>` |

`data-attr-<nome>` conta como o atributo (`data-attr-aria-label="t.titulo"`).

## `dist/`

```
dist/
  client/                  HTML por rota, assets com hash, /_pragma/manifest.json
  server/worker.js         Cloudflare Workers (se houver *.server.js)
  server/wrangler.jsonc
  server/node.mjs          Node: node dist/server/node.mjs  (porta: PORT, padrão 3000)
  templates/<rota>.json    rotas com ssr
```

- Sem `*.server.js`: `dist/client/` vai para hospedagem estática (rotas com `[param]` precisam
  de rewrite no host).
- Cloudflare: publique com o `dist/server/wrangler.jsonc`.
- Node: `node dist/server/node.mjs`.
- Caminho que termina em `.html` nunca é página SSR: é o HTML pré-gerado que o router busca.
