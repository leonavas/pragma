import { test } from 'node:test'
import assert from 'node:assert/strict'
import { render, renderHead, renderPage, serverServices } from '../../server/render.js'
import { template } from '../../contract.js'

const TPL = '<ul><!--p:for t1 item data.items item.id-->'
const EMPTY = '<!--p:empty t1-->'

const forOps = [
  ['html', TPL],
  ['for', 't1', 'item', 'data.items', 'item.id', [['html', '<li'], ['attr', 'title', 'item.name'], ['html', '>'], ['text', '{item.name}'], ['html', '</li>']]],
  ['html', EMPTY],
  ['empty', 't1', [['html', '<li>empty</li>']]],
  ['html', '</ul>'],
]

test('text and attr are escaped', () => {
  const ops = [['html', '<p'], ['attr', 'title', 'data.t'], ['html', '>'], ['text', '{data.t}'], ['html', '</p>']]
  const out = render(ops, { data: { t: `<script>"x"&'y'</script>` } })
  assert.equal(out, '<p title="&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;">&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;</p>')
})

test('attr: true is empty, false/null/undefined remove it', () => {
  const ops = [['html', '<input'], ['attr', 'disabled', 'data.a'], ['attr', 'x', 'data.b'], ['attr', 'y', 'data.c'], ['attr', 'z', 'data.d'], ['attr', 'n', 'data.n'], ['html', '>']]
  assert.equal(render(ops, { data: { a: true, b: false, c: null, d: undefined, n: 0 } }), '<input disabled="" n="0">')
})

test('text of null and undefined is empty', () => {
  assert.equal(render([['text', '{data.a}'], ['text', '{data.b}']], { data: { a: null, b: undefined } }), '')
})

test('text templates: literals are HTML source, values are escaped, \\{ is a brace', () => {
  const scope = { data: { name: 'Ana & <Bo>', n: 0 } }
  assert.equal(render([['text', 'Hi {data.name}, {data.n} items &amp; \\{more}']], scope), 'Hi Ana &amp; &lt;Bo&gt;, 0 items &amp; {more}')
  assert.deepEqual(template('a {x.y} \\{b} {z}'), ['a ', 'x.y', ' {b} ', 'z', ''])
})

test('attribute templates are always strings', () => {
  const ops = [['html', '<a'], ['attr', 'href', '/shop/{data.id}?q="{data.q}"'], ['attr', 'title', '{data.missing}'], ['html', '>']]
  assert.equal(render(ops, { data: { id: 7, q: 'a"b', missing: null } }), '<a href="/shop/7?q=&quot;a&quot;b&quot;" title="">')
})

test(':--name in style: values as the browser sets them; false, null, empty and ; { } < drop the property', () => {
  const ops = (style) => [['html', '<i'], ['attr', 'style', style], ['html', '></i>']]
  const data = { a: 12.5, b: 'red', no: false, nil: null, bad: '1;color:red', brace: '}a{', lt: '<x' }
  assert.equal(render(ops('--y:1;--a:{data.a};--b:{data.b}'), { data }), '<i style="--y:1;--a:12.5;--b:red"></i>')
  assert.equal(render(ops('--a:{data.no};--b:{data.b};--c:{data.nil}'), { data }), '<i style="--b:red"></i>')
  assert.equal(render(ops('--y:1;--a:{data.bad};--b:{data.brace};--c:{data.lt}'), { data }), '<i style="--y:1"></i>')
  assert.equal(render(ops('--a:{data.bad}'), { data }), '<i></i>')
})

test('browser-only services throw a clear error while the server renders', () => {
  const services = serverServices({ data: {}, params: {}, query: {}, page: {} }, null)
  for (const name of ['call', 'stream', 'navigate', 'open', 'close', 'confirm']) {
    assert.throws(() => services[name]('x'), { message: `${name} runs only in the browser` })
  }
})

test('markers beside elements: the server fills the text between them', () => {
  const ops = [['html', '<label><input> <!--{data.title}-->'], ['text', '{data.title}'], ['html', '<!--}--></label>']]
  assert.equal(render(ops, { data: { title: 'Buy <milk>' } }), '<label><input> <!--{data.title}-->Buy &lt;milk&gt;<!--}--></label>')
})

test('class merges static and conditional classes with negation', () => {
  const ops = [['html', '<div'], ['class', 'card p-4', [['active', 'data.on', false], ['empty', 'data.on', true]]], ['html', '>']]
  assert.equal(render(ops, { data: { on: true } }), '<div class="card p-4 active">')
  assert.equal(render(ops, { data: { on: false } }), '<div class="card p-4 empty">')
  assert.equal(render([['html', '<i'], ['class', '', [['a', 'data.on', false]]], ['html', '>']], { data: { on: false } }), '<i>')
})

test('show adds hidden when falsy', () => {
  const ops = [['html', '<p'], ['show', 'data.v', false], ['html', '>']]
  assert.equal(render(ops, { data: { v: false } }), '<p hidden>')
  assert.equal(render(ops, { data: { v: 1 } }), '<p>')
  assert.equal(render([['show', 'data.v', true]], { data: { v: 1 } }), ' hidden')
})

test('if/else-if/else: each instance goes right after its own template', () => {
  const IF = '<!--p:if t2 data.a-->'
  const ELIF = '<!--p:elif t2 !data.b-->'
  const ELSE = '<!--p:else t2-->'
  const ops = [
    ['html', IF],
    ['if', 't2', 'data.a', false, [['html', '\n  <b>a</b>\n']]],
    ['html', ELIF],
    ['elif', 't2', 'data.b', true, [['html', '<i>b</i>']]],
    ['html', ELSE],
    ['else', 't2', [['html', '<u>c</u>']]],
  ]
  assert.equal(render(ops, { data: { a: true, b: false } }), IF + '<b data-of="t2">a</b>' + ELIF + ELSE)
  assert.equal(render(ops, { data: { a: false, b: false } }), IF + ELIF + '<i data-of="t2">b</i>' + ELSE)
  assert.equal(render(ops, { data: { a: false, b: true } }), IF + ELIF + ELSE + '<u data-of="t2">c</u>')
  assert.equal(render([['if', 't3', 'data.ok', true, [['html', '<b>x</b>']]]], { data: { ok: true } }), '')
})

test('for renders instances with data-of and data-key; empty when the list is empty', () => {
  const out = render(forOps, { data: { items: [{ id: 1, name: 'a"b' }, { id: 'x<y', name: 'c' }] } })
  assert.equal(
    out,
    TPL + '<li data-of="t1" data-key="1" title="a&quot;b">a&quot;b</li><li data-of="t1" data-key="x&lt;y" title="c">c</li>' + EMPTY + '</ul>',
  )
  assert.equal(render(forOps, { data: { items: [] } }), TPL + EMPTY + '<li data-of="t1">empty</li></ul>')
})

test('nested for sees outer variables', () => {
  const ops = [['for', 'a', 'g', 'data.groups', 'g.id', [['html', '<ul>'], ['for', 'b', 'i', 'g.items', 'i', [['html', '<li>'], ['text', '{g.id}{i}'], ['html', '</li>']]], ['html', '</ul>']]]]
  const out = render(ops, { data: { groups: [{ id: 'G', items: ['1', '2'] }] } })
  assert.equal(out, '<ul data-of="a" data-key="G"><li data-of="b" data-key="1">G1</li><li data-of="b" data-key="2">G2</li></ul>')
})

test('unknown path throws', () => {
  assert.throws(() => render([['text', '{data.does.not.exist}']], { data: {} }), /unknown path: data\.does\.not\.exist/)
  assert.throws(() => render([['text', '{page.x}']], { data: {} }), /unknown path/)
  assert.equal(render([['text', '{data.s.length}']], { data: { s: 'abc' } }), '3')
})

test('duplicate key, invalid list and unknown op throw', () => {
  assert.throws(() => render(forOps, { data: { items: [{ id: 1, name: 'a' }, { id: 1, name: 'b' }] } }), /duplicate key/)
  assert.throws(() => render(forOps, { data: { items: 'abc' } }), /expects a list/)
  assert.throws(() => render([['nothing']], {}), /unknown op/)
})

test('state and shared are left to the client: directives intact, blocks without instances', () => {
  const ops = [
    ['html', '<p data-text="{state.n}"'], ['show', 'shared.ui.open', false], ['attr', 'title', 'state.t'], ['attr', 'alt', 'x {state.t} {data.ok}'],
    ['class', 'base', [['on', 'state.on', false], ['loaded', 'data.ok', false]]], ['html', '>'], ['text', '{state.n}'], ['text', 'n: {data.ok} {shared.cart.n}'], ['html', '</p>'],
    ['html', '<!--p:if t1 state.x-->'],
    ['if', 't1', 'state.x', false, [['html', '<b>x</b>']]],
    ['else', 't1', [['html', '<i>y</i>']]],
    ['html', '<!--p:for t2 i shared.cart.items i.id-->'],
    ['for', 't2', 'i', 'shared.cart.items', 'i.id', [['html', '<li>'], ['text', '{i.name}'], ['html', '</li>']]],
    ['empty', 't2', [['html', '<li>empty</li>']]],
  ]
  assert.equal(
    render(ops, { data: { ok: true } }),
    '<p data-text="{state.n}" class="base loaded"></p><!--p:if t1 state.x--><!--p:for t2 i shared.cart.items i.id-->',
  )
})

// Compiled script factories (what the build emits for `<script>` blocks).
const listScript = {
  default($, $pragma) {
    $pragma.prop('items', [])
    $pragma.prop('step', 1)
    $.open = true
    const count = () => $.items.length * $.step
    return { count }
  },
}
const LIST = (props) => [
  'comp', 'list', props, [
    ['html', '<ul data-c="list"'], ['class', '', [['open', 'state.open', false]]], ['html', '>'],
    ['for', 't1', 'item', 'state.items', 'item.id', [['html', '<li>'], ['text', '{item.name}'], ['html', '</li>']]],
    ['html', '<b>'], ['text', 'total {state.count}'], ['html', '</b></ul>'],
  ],
]

test('script components render with their instance: props (path or text), lets and derived functions', () => {
  const scope = { data: { items: [{ id: 1, name: 'one' }, { id: 2, name: 'two' }] }, params: {}, query: {}, page: {} }
  const out = renderPage([LIST([['items', 'path', 'data.items'], ['step', 'text', '3']])], scope, { components: { list: listScript } })
  assert.equal(out, '<ul data-c="list" class="open"><li data-of="t1" data-key="1">one</li><li data-of="t1" data-key="2">two</li><b>total 6</b></ul>')
})

test('a script component the server cannot run is left to the client (no module, browser-only code, stores, writes)', () => {
  const scope = { data: { items: [{ id: 1, name: 'one' }] }, params: {}, query: {}, page: {} }
  const client = '<ul data-c="list"></ul>'
  const props = [['items', 'path', 'data.items']]
  assert.equal(renderPage([LIST(props)], scope, {}), client.replace('></ul>', '><b></b></ul>'))
  const failing = [
    { default: () => { window.innerWidth } },
    { default: ($, $pragma) => { $.n = $pragma.shared.cart.items.length } },
    { default: ($, $pragma) => { $pragma.data.items.push({ id: 9 }) } },
    { default: ($, $pragma) => { $pragma.prop('items', []); return { count: () => $pragma.shared.cart.n } } },
  ]
  for (const list of failing) {
    assert.equal(renderPage([LIST(props)], scope, { components: { list } }), '<ul data-c="list"><b></b></ul>', String(list.default))
  }
  assert.equal(scope.data.items.length, 1, 'data stays untouched')
  // a prop from the client side (a store) leaves the whole component to the client
  assert.equal(renderPage([LIST([['items', 'path', 'shared.cart.items']])], scope, { components: { list: listScript } }), '<ul data-c="list"><b></b></ul>')
})

test('slot content renders in its owner scope; the page script renders too', () => {
  const page = { default($) { $.title = 'from page'; return { upper: () => 'PAGE' } } }
  const box = { default($) { $.title = 'from box' } }
  const ops = [
    ['html', '<h1>'], ['text', '{state.title} {state.upper}'], ['html', '</h1>'],
    ['comp', 'box', [], [
      ['html', '<section data-c="box"><h2>'], ['text', '{state.title}'], ['html', '</h2>'],
      ['slot', 1, [['html', '<p data-slot="1">'], ['text', '{state.title}'], ['html', '</p>']]],
      ['html', '<!--p:if1 t2 state.title-->'],
      ['slot', 1, [['if', 't2', 'state.title', false, [['html', '<i>'], ['text', '{state.title}'], ['html', '</i>']]]]],
      ['slot', 1, [['else', 't2', [['html', '<u>never</u>']]]]],
      ['html', '</section>'],
    ]],
  ]
  const out = renderPage(ops, { data: null, params: {}, query: {}, page: {} }, { page, components: { box } })
  assert.equal(out, '<h1>from page PAGE</h1><section data-c="box"><h2>from box</h2><p data-slot="1">from page</p><!--p:if1 t2 state.title--><i data-of="t2">from page</i></section>')
  // a page script that cannot run leaves page state to the client, still rendering the rest
  const broken = { default() { document.title } }
  assert.equal(renderPage([['html', '<h1>'], ['text', '{state.title}'], ['html', '</h1>']], { data: null, params: {}, query: {}, page: {} }, { page: broken }), '<h1></h1>')
})

test('for variable over data resolves on the server, even alongside state', () => {
  const ops = [['for', 't1', 'item', 'data.items', 'item.id', [['html', '<li'], ['class', '', [['sel', 'state.sel', false]]], ['html', '>'], ['text', '{item.name}'], ['html', '</li>']]]]
  assert.equal(render(ops, { data: { items: [{ id: 1, name: 'a' }] } }), '<li data-of="t1" data-key="1">a</li>')
  assert.throws(() => render([['for', 't1', 'item', 'data.items', 'item.id', [['html', '<li>'], ['text', '{item.missing}'], ['html', '</li>']]]], { data: { items: [{ id: 1 }] } }), /unknown path: item\.missing/)
  for (const path of ['data.x', 'params.x']) {
    assert.throws(() => render([['text', `{${path}}`]], { data: {}, params: {}, query: {} }), /unknown path/)
  }
})

test('null or undefined in the middle of a path is empty, as in the browser', () => {
  const scope = { data: { user: null, team: undefined }, params: {}, query: {}, page: {} }
  const ops = [['html', '<p'], ['attr', 'title', 'data.user.name'], ['show', 'data.team.open', false], ['html', '>'], ['text', 'a{data.user.name}b'], ['html', '</p>'], ['for', 't1', 'm', 'data.team.members', 'm.id', [['html', '<i>x</i>']]], ['empty', 't1', [['html', '<b>none</b>']]]]
  assert.equal(render(ops, scope), '<p hidden>ab</p><b data-of="t1">none</b>')
  assert.throws(() => render([['text', '{data.user.name}']], { data: { user: {} } }), /unknown path: data\.user\.name/)
})

test('a query param missing from the URL is empty, as in the browser', () => {
  const ops = [['html', '<p'], ['attr', 'title', 'query.q'], ['html', '>'], ['text', 'q={query.q}'], ['html', '</p>'], ['if', 't1', 'query.q', false, [['html', '<b>on</b>']]], ['else', 't1', [['html', '<i>off</i>']]]]
  assert.equal(render(ops, { data: null, params: {}, query: {}, page: {} }), '<p>q=</p><i data-of="t1">off</i>')
})

test('head: title and attribute templates and paths are filled by the server', () => {
  const html =
    '<html><head><title data-text="{data.p.name} &amp; co | Shop">Shop</title>\n' +
    '<meta property="og:title" data-attr-content="{data.p.name}">\n' +
    '<link rel="canonical" data-attr-href="https://x.dev/p/{params.id}">\n' +
    '<meta name="robots" data-attr-content="data.missing">\n' +
    '</head><body></body></html>'
  const out = renderHead(html, { data: { p: { name: 'Mug "XL"' } }, params: { id: '7' } })
  assert.equal(
    out,
    '<html><head><title>Mug &quot;XL&quot; &amp; co | Shop</title>\n' +
      '<meta property="og:title" content="Mug &quot;XL&quot;">\n' +
      '<link rel="canonical" href="https://x.dev/p/7">\n' +
      '</head><body></body></html>',
  )
})

test('head: JSON-LD gets {path} values JSON-escaped inside its strings; JSON syntax is kept', () => {
  const ld = '{"@context": "https://schema.org", "@type": "Article", "headline": "{data.post.title}", "author": {"@type": "Person", "name": "By { data.post.author }"}, "url": "https://x.dev/blog/{params.slug}", "note": "\\"{data.missing}\\""}'
  const html = `<html><head><title>Blog</title>\n<script type="application/ld+json">${ld}</script>\n</head><body><script type="application/ld+json">{"a": "{data.post.title}"}</script></body></html>`
  const scope = { data: { post: { title: 'He said "hi" </script><b>', author: 'Ana\nBo' } }, params: { slug: 'a-b' } }
  const out = renderHead(html, scope)
  const filled = out.slice(out.indexOf('<script type="application/ld+json">') + 35, out.indexOf('</script>'))
  assert.deepEqual(JSON.parse(filled), {
    '@context': 'https://schema.org',
    '@type': 'Article',
    headline: 'He said "hi" </script><b>',
    author: { '@type': 'Person', name: 'By Ana\nBo' },
    url: 'https://x.dev/blog/a-b',
    note: '""',
  })
  assert.ok(!filled.includes('</script>') && filled.includes('\\u003c/script>'), filled)
  assert.ok(out.endsWith('<body><script type="application/ld+json">{"a": "{data.post.title}"}</script></body></html>'), 'only the head is filled')
})
