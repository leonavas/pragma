import { test } from 'node:test'
import assert from 'node:assert/strict'
import { render, resolve } from '../../server/render.js'

const TPL = '<ul><template data-tid="t1" data-for="item in data.items" data-key="item.id"><li></li></template>'
const EMPTY = '<template data-empty data-tid="t9"><li>empty</li></template>'

const forOps = [
  ['html', TPL],
  ['for', 't1', 'item', 'data.items', 'item.id', [['html', '<li'], ['attr', 'title', 'item.name'], ['html', '>'], ['text', 'item.name'], ['html', '</li>']]],
  ['html', EMPTY],
  ['empty', 't1', [['html', '<li>empty</li>']]],
  ['html', '</ul>'],
]

test('text and attr are escaped', () => {
  const ops = [['html', '<p'], ['attr', 'title', 'data.t'], ['html', '>'], ['text', 'data.t'], ['html', '</p>']]
  const out = render(ops, { data: { t: `<script>"x"&'y'</script>` } })
  assert.equal(out, '<p title="&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;">&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;</p>')
})

test('attr: true is empty, false/null/undefined remove it', () => {
  const ops = [['html', '<input'], ['attr', 'disabled', 'data.a'], ['attr', 'x', 'data.b'], ['attr', 'y', 'data.c'], ['attr', 'z', 'data.d'], ['attr', 'n', 'data.n'], ['html', '>']]
  assert.equal(render(ops, { data: { a: true, b: false, c: null, d: undefined, n: 0 } }), '<input disabled="" n="0">')
})

test('text of null and undefined is empty', () => {
  assert.equal(render([['text', 'data.a'], ['text', 'data.b']], { data: { a: null, b: undefined } }), '')
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
  const IF = '<template data-tid="t2" data-if="data.a"><b>a</b></template>'
  const ELIF = '<template data-else-if="!data.b"><i>b</i></template>'
  const ELSE = '<template data-else><u>c</u></template>'
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
  const ops = [['for', 'a', 'g', 'data.groups', 'g.id', [['html', '<ul>'], ['for', 'b', 'i', 'g.items', 'i', [['html', '<li>'], ['text', 'g.id'], ['text', 'i'], ['html', '</li>']]], ['html', '</ul>']]]]
  const out = render(ops, { data: { groups: [{ id: 'G', items: ['1', '2'] }] } })
  assert.equal(out, '<ul data-of="a" data-key="G"><li data-of="b" data-key="1">G1</li><li data-of="b" data-key="2">G2</li></ul>')
})

test('unknown path throws', () => {
  assert.throws(() => render([['text', 'data.does.not.exist']], { data: {} }), /unknown path: data\.does\.not\.exist/)
  assert.throws(() => render([['text', 'page.x']], { data: {} }), /unknown path/)
  assert.throws(() => render([['show', 'data.a.b', false]], { data: { a: null } }), /unknown path/)
  assert.equal(resolve({ data: { s: 'abc' } }, 'data.s.length'), 3)
})

test('duplicate key, invalid list and unknown op throw', () => {
  assert.throws(() => render(forOps, { data: { items: [{ id: 1, name: 'a' }, { id: 1, name: 'b' }] } }), /duplicate key/)
  assert.throws(() => render(forOps, { data: { items: 'abc' } }), /expects a list/)
  assert.throws(() => render([['nothing']], {}), /unknown op/)
})

test('render is deterministic', () => {
  const data = () => ({ items: Array.from({ length: 50 }, (_, i) => ({ id: i, name: `item <${i}>` })) })
  const a = render(forOps, { data: data() })
  const b = render(forOps, { data: structuredClone(data()) })
  assert.equal(a, b)
  assert.equal(Buffer.compare(Buffer.from(a), Buffer.from(render(forOps, { data: data() }))), 0)
})

test('state and shared are left to the client: directives intact, blocks without instances', () => {
  const ops = [
    ['html', '<p data-text="state.n"'], ['show', 'shared.ui.open', false], ['attr', 'title', 'state.t'],
    ['class', 'base', [['on', 'state.on', false], ['loaded', 'data.ok', false]]], ['html', '>'], ['text', 'state.n'], ['html', '</p>'],
    ['html', '<template data-tid="t1" data-if="state.x"><b></b></template>'],
    ['if', 't1', 'state.x', false, [['html', '<b>x</b>']]],
    ['else', 't1', [['html', '<i>y</i>']]],
    ['html', '<template data-tid="t2" data-for="i in shared.cart.items" data-key="i.id"><li></li></template>'],
    ['for', 't2', 'i', 'shared.cart.items', 'i.id', [['html', '<li>'], ['text', 'i.name'], ['html', '</li>']]],
    ['empty', 't2', [['html', '<li>empty</li>']]],
  ]
  assert.equal(
    render(ops, { data: { ok: true } }),
    '<p data-text="state.n" class="base loaded"></p><template data-tid="t1" data-if="state.x"><b></b></template><template data-tid="t2" data-for="i in shared.cart.items" data-key="i.id"><li></li></template>',
  )
})

test('for variable over data resolves on the server, even alongside state', () => {
  const ops = [['for', 't1', 'item', 'data.items', 'item.id', [['html', '<li'], ['class', '', [['sel', 'state.sel', false]]], ['html', '>'], ['text', 'item.name'], ['html', '</li>']]]]
  assert.equal(render(ops, { data: { items: [{ id: 1, name: 'a' }] } }), '<li data-of="t1" data-key="1">a</li>')
  assert.throws(() => render([['for', 't1', 'item', 'data.items', 'item.id', [['html', '<li>'], ['text', 'item.missing'], ['html', '</li>']]]], { data: { items: [{ id: 1 }] } }), /unknown path: item\.missing/)
  for (const path of ['data.x', 'params.x', 'query.x']) {
    assert.throws(() => render([['text', path]], { data: {}, params: {}, query: {} }), /unknown path/)
  }
})
