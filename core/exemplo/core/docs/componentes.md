# Componentes

Reaproveite. Antes de escrever marcação nova, procure em `lib/components/` e
`routes/<pasta>/_components/`. Referência: `exemplo/lib/components/` (`contador`, `indicador`,
`abas`) e `exemplo/routes/loja/_components/comprar/`.

## Quando criar

- Usado em 2+ lugares, ou é uma unidade com nome claro (card, item, contador): componente.
- Usado só numa pasta de rotas: `routes/<pasta>/_components/<nome>/`.
- Usado em mais de uma pasta: `lib/components/<nome>/`.
- Resolução de `<c-nome>`: primeiro `_components/` da rota, depois `lib/components/`.

## Arquivos

```
lib/components/contador/
  contador.html   obrigatório, 1 elemento raiz (não <template>, <slot> nem <c-x>)
  contador.js     opcional: state, actions, mount
  contador.css    opcional, escopo [data-c="contador"]
```

## Exemplo canônico: com `.js` e props

`lib/components/contador/contador.html`:
```html
<div class="inline-flex items-center gap-2" role="group" data-attr-aria-label="props.rotulo">
  <span class="text-sm text-muted" data-text="props.rotulo"></span>
  <button type="button" class="btn" aria-label="Diminuir" data-on-click="menos" data-attr-disabled="state.semMenos">−</button>
  <output class="w-8 text-center tabular-nums" data-text="state.n"></output>
  <button type="button" class="btn" aria-label="Aumentar" data-on-click="mais">+</button>
</div>
```

`contador.js`:
```js
export const state = ({ props }) => ({
  n: Number(props.inicial || 0),
  get passo() { return Number(props.passo || 1) },
  get semMenos() { return this.n - this.passo < 0 },
})

export const actions = {
  menos(ctx) { ctx.state.n -= ctx.state.passo },
  mais(ctx) { ctx.state.n += ctx.state.passo },
}
```

Uso: `<c-contador rotulo="Pulos de 5" inicial="10" passo="5"></c-contador>`

## Exemplo canônico: dados de runtime e slots

`lib/components/indicador/indicador.html` (sem `.js`):
```html
<article class="card flex flex-col gap-1" data-attr-data-tom="tom">
  <h3 class="text-sm text-muted" data-text="props.titulo"></h3>
  <p class="text-2xl font-semibold" data-text="valor"></p>
  <slot></slot>
</article>
```

Uso:
```html
<c-indicador titulo="Pendentes" data-with-valor="shared.tarefas.pendentes" data-with-tom="state.tom">
  <a href="/tarefas">Ver tarefas</a>
</c-indicador>
```

Valor de runtime para um componente com `.js` (`exemplo/routes/loja/_components/produto-card/`):
```html
<c-comprar data-attr-data-id="p.id" data-attr-data-estoque="p.estoque"></c-comprar>
```
No `comprar.js`: `ctx.props.id`, `Number(ctx.props.estoque)`.

## Props

- Prop = atributo do `<c-x>`. Sempre texto. `props.nome` vira literal no build.
- Prop ausente = vazio e falso: `<template data-if="props.dica">` some no build.
- Componente com `.js`: o build copia as props (menos `class` e `id`) para a raiz como
  `data-<prop>`; o JS lê `ctx.props.<prop>` (strings; `max-itens` → `ctx.props.maxItens`),
  também no `state(ctx)`.
- Valor de runtime para o JS: `data-attr-data-<prop>="caminho"` no `<c-x>`, avaliado no escopo
  de quem usa, antes do `state(ctx)`. Caminho de `data`, `shared` ou variável do `data-for`.
- `class` no `<c-x>` soma às classes da raiz. `id` é prop: o componente o aplica
  (`data-attr-id="props.id"`).

## Regras

- Dados de runtime para o HTML: alias no componente, `data-with-<alias>="caminho"` em quem usa.
- Caminhos de runtime dentro do componente resolvem no escopo de quem usa.
- `data-with-<alias>="state.x"` em componente com `.js` = erro: lá dentro `state` é o do
  componente. Use `data-attr-data-*` ou um store.
- `<slot></slot>` recebe os filhos; `<slot name="acoes">` recebe filhos com `slot="acoes"`.
  Filho para slot inexistente = erro.
- Conteúdo de slot é de quem o escreveu: suas actions e `state` são da página (ver
  `diretivas.md`, dono léxico).
- `state` de componente é por instância.
- `data-on-*` no `.html` de componente sem `.js` chama a action de quem usa.
- O build marca a raiz com `data-c="<nome>"`. O CSS do componente usa esse escopo.
- O `.js` do componente só é baixado nas páginas que o usam.
- Componente não tem `.server.js`; dados vêm do `load` da página.

## Não faça

- Não copie e cole marcação repetida entre páginas: extraia componente.
- Não crie componente de uma linha só para envolver uma classe; use classe custom (ver `css.md`).
- Não use custom elements (`customElements.define`) nem shadow DOM.
- Não leia nem mude o estado de um componente filho pela página: o pai passa props, o filho
  muda store ou chama action própria.
- Não escreva overlays do zero: use `c-modal`, `c-alert`, `c-dropdown` (ver `overlays.md`).
