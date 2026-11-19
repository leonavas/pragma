# Overlays: modal, alert, dropdown

Sempre `c-modal`, `c-alert`, `c-dropdown`. O runtime cuida de abrir, fechar, Esc, voltar, foco
e trava de scroll.

```html
<button type="button" class="btn" @open="sobre">Sobre</button>
<c-modal id="sobre" titulo="Sobre o app">
  <p>Conteúdo.</p>
  <button slot="acoes" type="button" class="btn btn-primary" @close value="ok">Fechar</button>
</c-modal>

<button type="button" class="btn" @click="apagar">Apagar</button>
<c-alert id="apagar" titulo="Apagar projeto?">
  <p>Essa ação não pode ser desfeita.</p>
  <button slot="acoes" type="button" class="btn" @close value="cancel" autofocus>Cancelar</button>
  <button slot="acoes" type="button" class="btn btn-danger" @close value="ok">Apagar</button>
</c-alert>

<c-dropdown id="conta" rotulo="Conta">
  <a href="/perfil">Perfil</a>
  <hr>
  <button type="button" @click="sair">Sair</button>
</c-dropdown>
```

```js
export const actions = {
  async apagar(ctx) {
    if (await ctx.confirm('apagar') !== 'ok') return
    await ctx.call('apagar', { id: ctx.params.id })
  },
}
```

| Componente | Props | Slots | Fecha com |
|---|---|---|---|
| `c-modal` | `id`, `titulo` | padrão, `acoes` | Esc, fundo, ×, `@close`, voltar |
| `c-alert` | `id`, `titulo` | padrão, `acoes` | Esc, `@close`, voltar |
| `c-dropdown` | `id`, `rotulo` | padrão (`<a>`, `<button>`, `<hr>`) | clique fora, Esc, `@close`, voltar |

| JS | Efeito |
|---|---|
| `ctx.open(id)` / `ctx.close(id)` | abre / fecha |
| `await ctx.confirm(id)` | abre e resolve com o `value` do `@close` clicado, ou `'cancel'` |

## Regras

- Voltar fecha o overlay do topo, não a página. Overlays empilham.
- Modal e alert travam o scroll; dropdown não. O foco volta a quem abriu.
- Ação destrutiva: `autofocus` no botão seguro.
- Abra só em resposta a clique do usuário.
- Não crie `<dialog>`/`[popover]` à mão, não chame `showModal()`, não use `window.confirm`,
  não guarde "aberto" no estado, não use `role="menu"`, não importe `core/overlay.js`.
