# Estrutura do app

Fixa. Não crie pastas fora dela nem edite `core/`.

```
<app>/
  pragma.json              { "name": "app", "budgets": { "initialJsKb": 15, "routeJsKb": 10, "cssKb": 15 } }
  package.json             pacotes npm em dependencies (opcional, ver pacotes.md)
  routes/
    _layout.html  _404.html
    index.html             página /
    index.js               comportamento (opcional)
    index.server.js        dados (opcional)
    loja/
      [id].html            /loja/:id
      _catalogo.server.js  helper privado de servidor
      _produtos.json       dado privado
      _components/<nome>/  <p-nome>
  lib/
    components/<nome>/     <c-nome> (modal, alert e dropdown vêm prontos)
    state/<nome>.js        stores
    theme.css              tokens e classes custom
  utils/                   funções puras
  public/                  estáticos (imagens, fontes, robots.txt)
  core/                    framework, não edite
```

| Coisa | Lugar |
|---|---|
| Página | `routes/<caminho>.html` + `.js` + `.server.js` (mesmo nome) |
| Helper ou JSON só do servidor | `routes/<pasta>/_nome.server.js`, `_nome.json` |
| Componente de uma pasta | `routes/<pasta>/_components/<nome>/` |
| Componente compartilhado | `lib/components/<nome>/` |
| Estado entre páginas | `lib/state/<nome>.js` |
| Token, cor, classe global | `lib/theme.css` |
| Função pura | `utils/<nome>.js` |

- `_` no início = privado, não é rota. Só `_layout.html` e `_404.html` são especiais.
- Sem `<script>` em `routes/` ou `lib/`, sem `src/`, `pages/`, `styles/`, `helpers/`.
- Pacote npm só de JS puro (ES module), em `dependencies`; veja `pacotes.md`.
- CSS em componente ou `lib/theme.css`; estático em `public/`.
