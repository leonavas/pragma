# Overlays: modal, alert, dropdown

Use sempre os componentes prontos de `lib/components/`. O runtime (`core/overlay.js`) cuida de
abrir, fechar, Esc, clique fora, botão voltar, foco e trava de scroll. Referência:
`exemplo/routes/tarefas/index.html` (dropdown + alert) e `exemplo/routes/index.html` (modal).

## Regras

- Abrir: `<button data-open="<id>">`. Fechar: `<button data-close>` dentro do overlay.
- O `id` do overlay é único na página e vem da prop `id`.
- Voltar (browser ou Android) fecha o overlay do topo e não sai da página.
- Modal fecha com Esc, clique no fundo, `data-close` ou voltar.
- Alert fecha com `data-close`, Esc ou voltar. Clique no fundo não fecha.
- Dropdown fecha com clique fora, Esc, `data-close`, voltar ou clicando no botão de novo.
- Modal e alert travam o scroll da página até o último fechar, sem salto de layout
  (`html[data-scroll-lock]`; `="gutter"` reserva a barra se havia). Dropdown não trava.
- O foco volta ao elemento que abriu.
- Overlay aberto por cima de outro empilha; Esc e voltar fecham um por vez, do topo.

## Modal

```html
<button type="button" class="btn" data-open="sobre">Sobre</button>

<c-modal id="sobre" titulo="Sobre o app">
  <p>Conteúdo.</p>
  <button slot="acoes" type="button" class="btn btn-primary" data-close value="ok">Fechar</button>
</c-modal>
```

Props: `id`, `titulo` (vira o título visível e o nome acessível). Slots: padrão (corpo) e
`acoes` (rodapé). O botão de fechar (×) já vem no componente.

## Alert (confirmação)

```html
<button type="button" class="btn" data-on-click="apagar">Apagar</button>

<c-alert id="apagar" titulo="Apagar projeto?">
  <p>Essa ação não pode ser desfeita.</p>
  <button slot="acoes" type="button" class="btn" data-close value="cancel" autofocus>Cancelar</button>
  <button slot="acoes" type="button" class="btn btn-danger" data-close value="ok">Apagar</button>
</c-alert>
```

```js
export const actions = {
  async apagar(ctx) {
    if (await ctx.confirm('apagar') !== 'ok') return
    await ctx.call('apagar', { id: ctx.params.id })
  },
}
```

Props: `id`, `titulo`. Slots: padrão (mensagem) e `acoes`. Em ação destrutiva, ponha
`autofocus` no botão seguro.

## Dropdown

```html
<c-dropdown id="conta" rotulo="Conta">
  <a href="/perfil">Perfil</a>
  <a href="/config">Configurações</a>
  <hr>
  <button type="button" data-on-click="sair">Sair</button>
</c-dropdown>
```

Props: `id`, `rotulo` (texto do botão). Slot padrão: `<a>`, `<button>` e `<hr>` diretos; o
estilo de item já vem do componente. O menu abre colado ao botão. `data-close` no item fecha o
menu ao clicar; actions dos itens são de quem escreveu o dropdown.

## API no JS (`ctx`)

| Chamada | Efeito |
|---|---|
| `ctx.open(id)` | abre o overlay |
| `ctx.close(id)` | fecha o overlay |
| `await ctx.confirm(id)` | abre e espera fechar; resolve com o `value` do botão `data-close` clicado, ou `'cancel'` se fechou de outro jeito (Esc, fundo, voltar, ×, botão sem `value`) |

## Faça

- Faça `data-open` em `<button type="button">`.
- Faça confirmação com `c-alert` + `await ctx.confirm(id)`.
- Faça o conteúdo do overlay com diretivas normais (`data-text`, `data-for`…) ligadas ao estado
  da página.
- Abra overlays em resposta a um clique do usuário. Aberto sem interação (timer, ao carregar),
  o Chrome pode pular a entrada no histórico e o voltar sai da página.

## Não faça

- Não crie `<dialog>` ou `[popover]` à mão; use `c-modal`, `c-alert` e `c-dropdown`.
- Não chame `showModal()`, `close()`, `showPopover()` nem mexa em `history` para overlays.
- Não use `window.confirm`, `alert` ou `prompt`.
- Não ponha `data-close` fora de um overlay.
- Não use `role="menu"`: o dropdown é uma lista de links e botões navegável por Tab.
- Não guarde "aberto/fechado" em `state` para controlar overlay.
- Não importe `core/overlay.js`: o runtime carrega só em página com overlay.
