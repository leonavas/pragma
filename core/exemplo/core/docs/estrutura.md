# Estrutura do app

A estrutura é fixa. Não crie pastas fora dela.

```
<app>/
  AGENTS.md  CLAUDE.md
  pragma.json            { "name": "app", "budgets": { "initialJsKb": 15, "routeJsKb": 10, "cssKb": 15 } }
  routes/
    _layout.html         layout raiz, com <main data-page><slot></slot></main>
    _404.html
    index.html           página  → /
    index.js             comportamento da página (opcional)
    index.server.js      dados da página (opcional)
    loja/
      index.html         → /loja
      [id].html          → /loja/:id
      [id].js  [id].server.js
      _catalogo.server.js  helper de servidor privado (não é rota)
      _produtos.json       dado privado, lido por import no .server.js
      _components/<nome>/<nome>.html|.js|.css   componentes só desta pasta de rotas
  lib/
    components/<nome>/<nome>.html|.js|.css      componentes do app (modal, alert e dropdown vêm prontos)
    state/<nome>.js      stores compartilhados
    theme.css            tokens e classes custom globais
  utils/                 funções puras
  public/                arquivos servidos como estão (imagens, fontes, robots.txt)
  core/                  framework; gerado, não edite (state.js, theme.js, log.js…)
```

Referência completa: `exemplo/` (Mercadinho), o app de exemplo no repositório do pragma. Os
docs citam arquivos dele como `exemplo/<caminho>`.

## Onde colocar

| Coisa | Lugar |
|---|---|
| Página | `routes/<caminho>.html` |
| Comportamento da página | `routes/<caminho>.js` (mesmo nome do `.html`) |
| Dados e ações de servidor | `routes/<caminho>.server.js` |
| Helper ou JSON só do servidor de uma pasta de rotas | `routes/<pasta>/_nome.server.js`, `_nome.json` |
| Componente de uma pasta de rotas | `routes/<pasta>/_components/<nome>/` |
| Componente usado em 2+ lugares | `lib/components/<nome>/` |
| Estado compartilhado entre páginas | `lib/state/<nome>.js` |
| Token, cor, classe custom global | `lib/theme.css` |
| Função pura (formatar, calcular) | `utils/<nome>.js` |

## Não faça

- Não edite nada em `core/`.
- Não crie `<script>` em `routes/` ou `lib/`; comportamento vai no `.js` irmão.
- Não importe pacotes npm. Imports só de arquivos do projeto e de `core/`.
- Não crie pastas como `src/`, `pages/`, `styles/` ou `helpers/`.
- Não use `_` no início de nomes de página: `_` marca arquivos privados (não são rota).
  `_layout.html` e `_404.html` são os únicos especiais.
- Não ponha CSS, imagem ou outro estático em `routes/`: CSS no componente ou `lib/theme.css`,
  estáticos em `public/`.
