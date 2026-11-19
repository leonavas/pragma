# Componentes

Procure em `lib/components/` e `_components/` antes de criar.

| Tag | Pasta | Quando |
|---|---|---|
| `<c-nome>` | `lib/components/nome/` | usado em mais de uma pasta de rotas |
| `<p-nome>` | `_components/nome/` da pasta da página ou acima | usado só naquela pasta |

Promover: mova a pasta para `lib/components/` e troque `p-` por `c-`.

```
nome/
  nome.html   obrigatório, 1 elemento raiz (não <template>, <slot> nem componente)
  nome.js     opcional: state, actions, mount
  nome.css    opcional, escopo [data-c="nome"]
```

## Props

| Quem usa escreve | Dentro do componente |
|---|---|
| `titulo="Oi"` (fixa) | `$titulo` no HTML, `ctx.$.titulo` no JS |
| `:total="$total"` (dinâmica, atualiza sozinha) | `$total`, `ctx.$.total` |

- Prop é só leitura. Prop ausente = vazio e falso. Prop com nome de chave do estado = erro.
- Componente com `.js` não recebe `$` de quem usa: passe `data`, `shared` ou item do `:for`,
  ou use um store.
- `class` soma às classes da raiz. `id` é prop: aplique com `:id="$id"`.

## Exemplo

`lib/components/contador/contador.html`:
```html
<div class="inline-flex items-center gap-2" role="group" :aria-label="$rotulo">
  <span class="text-sm text-muted" :text="$rotulo"></span>
  <button type="button" class="btn" aria-label="Diminuir" @click="menos" :disabled="$semMenos">−</button>
  <output class="w-8 text-center tabular-nums" :text="$quantidade"></output>
  <button type="button" class="btn" aria-label="Aumentar" @click="mais">+</button>
</div>
```

`contador.js`:
```js
export const state = ctx => ({
  quantidade: Number(ctx.$.inicial || 0),
  get passo() { return Number(ctx.$.passo || 1) },
  get semMenos() { return this.quantidade - this.passo < 0 },
})

export const actions = {
  menos(ctx) { ctx.$.quantidade -= ctx.$.passo },
  mais(ctx) { ctx.$.quantidade += ctx.$.passo },
}
```

Uso:
```html
<c-contador rotulo="Pulos de 5" inicial="10" passo="5"></c-contador>
<template :for="p in data.produtos" :key="p.id">
  <p-produto-card :produto="p"></p-produto-card>
</template>
```

## Slots

```html
<c-modal id="sobre" titulo="Sobre">
  <p>Corpo vai no slot padrão.</p>
  <button slot="acoes" type="button" class="btn" @close>Fechar</button>
</c-modal>
```

- `<slot></slot>` recebe os filhos; `<slot name="acoes">` recebe `slot="acoes"`. Slot inexistente = erro.
- Conteúdo de slot é de quem o escreveu: `$` e `@` dele.

## Regras

- Componente sem `.js`: `@` chama a action de quem usa.
- Estado de componente é por instância. O `.js` só baixa nas páginas que o usam.
- Sem `.server.js`: dados vêm do `load` da página.
- Não leia nem mude o estado de um filho: passe props; o filho muda store ou chama action própria.
- Sem `customElements.define` e sem shadow DOM. Overlay: use `c-modal`, `c-alert`, `c-dropdown`.
