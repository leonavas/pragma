# Sintaxe do HTML

`:` liga valor, `.` liga classe, `@` liga evento, `$` é estado ou prop. Lista fechada: o resto é erro.

| Escreva | Onde | Efeito |
|---|---|---|
| `:text="v"` | elemento | texto (escapado) |
| `:<atributo>="v"` | elemento | atributo (`:href`, `:disabled`, `:aria-current`); `false`/`null` remove, `true` = vazio |
| `.<classe>="[!]v"` | elemento | liga/desliga a classe |
| `:show="[!]v"` | elemento | `hidden` quando falso |
| `:if="[!]v"` / `:else` | `<template>` | cria/remove o bloco; `:else` logo depois do `:if` |
| `:for="t in v"` + `:key="t.id"` | `<template>` | repete; `:key` obrigatório, id estável |
| `:empty` | `<template>` logo depois do `:for` | lista vazia |
| `:model="$v"` | input, select, textarea | mão dupla; só `$` ou `shared` |
| `@<evento>="acao"` | elemento | chama `actions.acao` do dono |
| `@open="id"` / `@close` | botão | abre/fecha overlay |

## Valores

Só caminho `raiz.a.b`: sem expressão, chamada, índice ou operador. `!` só em `:if`, `:show` e `.classe`.

| Raiz | Valor |
|---|---|
| `$x` | estado ou prop do dono |
| `data` | retorno do `load` |
| `shared` | stores (`shared.carrinho.total`) |
| `params` `query` | path params e query (strings) |
| `page` | `page.loading`, `page.error` |
| variável do `:for` | item atual |

Valor calculado (total, rótulo, filtro, flag) é getter no estado, nunca no HTML.

## Exemplo

```html
<form class="flex gap-2" @submit="adicionar">
  <input class="flex-1 rounded-lg border border-line px-3 py-2" aria-label="Nova tarefa" :model="$nova">
  <button class="btn btn-primary">Adicionar</button>
</form>
<p class="text-sm text-danger-600" role="status" :show="$erro" :text="$erro"></p>
<ul>
  <template :for="t in $visiveis" :key="t.id">
    <li .line-through="t.feita">
      <input type="checkbox" :checked="t.feita" :aria-label="t.titulo" @change="alternar">
      <span :text="t.titulo"></span>
    </li>
  </template>
  <template :empty>
    <li class="text-muted">Nenhuma tarefa.</li>
  </template>
</ul>
```

## Dono

- Dono = quem escreveu o elemento: a página, ou o componente com `.js` em cujo `.html` ele está.
- `$`, `@` e `:model` usam o dono. Conteúdo de slot é de quem o escreveu.
- No `_layout.html` não há dono: só `shared`, `params` e `query`.

## Regras

- `<template>` com `:if`, `:else`, `:for` ou `:empty` tem 1 elemento raiz.
- HTML estrito: feche toda tag não void. Sem `<html>`, `<body>`, `<script>`, `<style>`.
- `@submit` já faz `preventDefault`. Na action, o item do `:for` está em `ctx.scope.t`.
- `aria-*` com valor: getter devolve `'true'`/`'false'` (string).
- `:show` alterna algo que existe; `:if` cria/remove. Bloco novo com `autofocus` ganha foco.
- Caminho inexistente em dev = erro.
- Marcadores: `data-page` (layout), `data-reload` (`<a>` com carga completa). Não escreva
  `data-c`, `data-tid`, `data-of`, `data-slot`: são gerados.
- Proibido: `style`, `on*` inline, `{{ }}`, `${ }` e a sintaxe antiga `data-text`, `data-on-*`,
  `data-attr-*`, `data-class-*`, `data-with-*`, `state.`, `props.`.
