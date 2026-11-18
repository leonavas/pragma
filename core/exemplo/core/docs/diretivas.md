# Diretivas

Lista fechada. O que não está aqui é erro no `pragma check`.
Referência completa: `exemplo/routes/tarefas/index.html`.

## HTML estrito

- Feche toda tag não void: `</p>`, `</li>`, `</td>`, `</option>`. Sem fechamento = erro `html/sintaxe`.
- Void não fecha: `<input>`, `<img>`, `<br>`, `<hr>`, `<meta>`, `<link>`, `<source>`.
- Não escreva `<html>`, `<body>`, `<script>`, `<style>`. `<head>` só no início do `_layout.html`;
  `<title>` só no topo da página.

## Caminhos

Só `raiz.a.b`. Sem expressões, chamadas, índices, operadores ou template strings.

| Raiz | Valor |
|---|---|
| `data` | retorno do `load` da página |
| `state` | estado do dono léxico (página ou componente com `.js`) |
| `shared` | stores de `lib/state/<nome>.js` (`shared.carrinho.itens`) |
| `params` `query` | path params e query string (strings) |
| `page` | `page.loading`, `page.error` |
| `props` | atributos do `<c-x>`, resolvidos no build (só em componente) |
| variável do `data-for` / alias do `data-with-*` | escopo do bloco |

Valor calculado (total, rótulo, filtro, "tem itens"): getter no `state` ou no store (ver
`estado.md`). Nunca calcule no HTML.

## Lista

| Diretiva | Onde | Efeito |
|---|---|---|
| `data-text="p"` | elemento | `textContent` (sempre escapado) |
| `data-attr-<nome>="p"` | elemento | atributo; `false`/`null`/`undefined` remove; `true` = vazio |
| `data-class-<nome>="[!]p"` | elemento | liga/desliga a classe `<nome>` |
| `data-show="[!]p"` | elemento | `hidden` quando falso |
| `data-if="[!]p"` | `<template>` | renderiza se verdadeiro |
| `data-else` | `<template>` logo após `data-if` | senão |
| `data-for="x in p"` + `data-key="x.campo"` | `<template>` | repete; `data-key` obrigatório |
| `data-empty` | `<template>` logo após `data-for` | lista vazia |
| `data-on-<evento>="acao"` | elemento | chama `actions.acao` do dono léxico |
| `data-model="state.p"` | input, select, textarea | mão dupla; raiz `state` ou `shared` |
| `data-with-<alias>="p"` | `<c-x>` | dentro do componente, `<alias>` = `p` |
| `data-open="id"` / `data-close` | botão | abre/fecha overlay (ver `overlays.md`) |
| `data-reload` | `<a>` | carga completa, sem router |

## Exemplo canônico

`routes/tarefas/index.html` (JS em `estado.md`):
```html
<title>Tarefas</title>
<section class="flex flex-col gap-4">
  <h1 class="text-2xl font-semibold">Tarefas</h1>
  <form class="flex gap-2" data-on-submit="adicionar">
    <input class="flex-1 rounded-lg border border-line px-3 py-2" aria-label="Nova tarefa" data-model="state.nova">
    <button class="btn btn-primary">Adicionar</button>
  </form>
  <p class="text-sm text-danger-600" role="status" data-text="state.erro"></p>
  <ul class="flex flex-col gap-1">
    <template data-for="t in state.visiveis" data-key="t.id">
      <li class="flex items-center gap-2">
        <input type="checkbox" data-attr-checked="t.feita" data-attr-aria-label="t.titulo" data-on-change="alternar">
        <span data-class-line-through="t.feita" data-text="t.titulo"></span>
      </li>
    </template>
    <template data-empty>
      <li class="text-muted">Nenhuma tarefa.</li>
    </template>
  </ul>
</section>
```

## Dono léxico

- O dono de `data-on-*`, `data-model` e `state` é quem escreveu o elemento: no `.html` de um
  componente com `.js`, o componente; em qualquer outro lugar, a página.
- Conteúdo de slot é de quem o escreveu. Botão passado para `<c-modal>` pela página chama
  action da página e lê o `state` da página.
- A raiz de um componente com `.js` é do componente.
- No layout não há dono: `data-on-*` no layout só dentro de componente com `.js`.
- O build marca conteúdo de slot com `data-slot="n"`. Não escreva `data-slot`, `data-c`,
  `data-tid` nem `data-of`: são gerados.

## Regras

- `<template data-if|else|for|empty>` contém exatamente 1 elemento raiz, que não é `<template>`.
- `data-if` e `data-for` só em `<template>`. `data-else` e `data-empty` logo depois do par.
- `!` só em `data-if`, `data-show` e `data-class-*`.
- `data-key` parte da variável do `data-for` e é um campo único e estável (id), nunca o índice.
- Caminho inexistente em dev = erro. Não conte com vazio silencioso.
- `data-show` para alternar algo que já existe; `data-if` para criar/remover bloco.
- Bloco recém-inserido com `[autofocus]` recebe o foco (edição inline).
- `aria-*` com valor (`aria-pressed`, `aria-invalid`, `aria-current`): o getter devolve string
  (`'true'`, `'false'`); `true` booleano vira atributo vazio, `false` remove.
- `data-attr-checked` e `data-attr-selected` também ligam a propriedade.
- `data-model` em componente aceita alias, desde que aponte para `state` ou `shared`.
- `props` vale em `data-text`, `data-attr-*`, `data-class-*`, `data-show` e `data-if`; não em
  `data-model`, `data-for` nem `data-with-*`.
- `data-on-submit` já faz `preventDefault`.
- Dentro de `data-for`, a action recebe o item em `ctx.scope` (`ctx.scope.t`).

## Não faça

- Não use `style="..."`, `on*` inline, `data-attr-style` nem `data-attr-on*`.
- Não use `{{ }}`, `${ }`, `x-*`, `v-*`, `:attr` ou qualquer sintaxe de outro framework.
- Não escreva no DOM pelo JS (`textContent`, `classList`, `setAttribute`): mude o estado e
  deixe a diretiva aplicar.
