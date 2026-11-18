import { test } from 'node:test'
import assert from 'node:assert/strict'
import { render, resolve } from '../../server/render.js'

const TPL = '<ul><template data-tid="t1" data-for="item in data.itens" data-key="item.id"><li></li></template>'

const forOps = [
  ['html', TPL],
  ['for', 't1', 'item', 'data.itens', 'item.id', [['html', '<li'], ['attr', 'title', 'item.nome'], ['html', '>'], ['text', 'item.nome'], ['html', '</li>']], [['html', '<li>vazio</li>']]],
  ['html', '</ul>'],
]

test('texto e atributo saem escapados', () => {
  const ops = [['html', '<p'], ['attr', 'title', 'data.t'], ['html', '>'], ['text', 'data.t'], ['html', '</p>']]
  const out = render(ops, { data: { t: `<script>"x"&'y'</script>` } })
  assert.equal(out, '<p title="&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;">&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;</p>')
})

test('attr: true vazio, false/null/undefined removem', () => {
  const ops = [['html', '<input'], ['attr', 'disabled', 'data.a'], ['attr', 'x', 'data.b'], ['attr', 'y', 'data.c'], ['attr', 'z', 'data.d'], ['attr', 'n', 'data.n'], ['html', '>']]
  assert.equal(render(ops, { data: { a: true, b: false, c: null, d: undefined, n: 0 } }), '<input disabled="" n="0">')
})

test('text de null e undefined fica vazio', () => {
  assert.equal(render([['text', 'data.a'], ['text', 'data.b']], { data: { a: null, b: undefined } }), '')
})

test('class combina estáticas e condicionais com negação', () => {
  const ops = [['html', '<div'], ['class', 'card p-4', [['ativo', 'data.on', false], ['vazio', 'data.on', true]]], ['html', '>']]
  assert.equal(render(ops, { data: { on: true } }), '<div class="card p-4 ativo">')
  assert.equal(render(ops, { data: { on: false } }), '<div class="card p-4 vazio">')
  assert.equal(render([['html', '<i'], ['class', '', [['a', 'data.on', false]]], ['html', '>']], { data: { on: false } }), '<i>')
})

test('show põe hidden quando falso', () => {
  const ops = [['html', '<p'], ['show', 'data.v', false], ['html', '>']]
  assert.equal(render(ops, { data: { v: false } }), '<p hidden>')
  assert.equal(render(ops, { data: { v: 1 } }), '<p>')
  assert.equal(render([['show', 'data.v', true]], { data: { v: 1 } }), ' hidden')
})

test('if/else gera instância com data-of logo depois do template', () => {
  const ops = [
    ['html', '<template data-tid="t2" data-if="data.ok"><b>sim</b></template><template data-else><i>não</i></template>'],
    ['if', 't2', 'data.ok', false, [['html', '\n  <b>sim</b>\n']], [['html', '<i>não</i>']]],
  ]
  assert.equal(render(ops, { data: { ok: true } }), ops[0][1] + '<b data-of="t2">sim</b>')
  assert.equal(render(ops, { data: { ok: false } }), ops[0][1] + '<i data-of="t2">não</i>')
  assert.equal(render([['if', 't3', 'data.ok', true, [['html', '<b>x</b>']], []]], { data: { ok: true } }), '')
})

test('for gera instâncias com data-of e data-key; empty quando vazio', () => {
  const out = render(forOps, { data: { itens: [{ id: 1, nome: 'a"b' }, { id: 'x<y', nome: 'c' }] } })
  assert.equal(
    out,
    TPL + '<li data-of="t1" data-key="1" title="a&quot;b">a&quot;b</li><li data-of="t1" data-key="x&lt;y" title="c">c</li></ul>',
  )
  assert.equal(render(forOps, { data: { itens: [] } }), TPL + '<li data-of="t1">vazio</li></ul>')
})

test('for aninhado enxerga as variáveis externas', () => {
  const ops = [['for', 'a', 'g', 'data.grupos', 'g.id', [['html', '<ul>'], ['for', 'b', 'i', 'g.itens', 'i', [['html', '<li>'], ['text', 'g.id'], ['text', 'i'], ['html', '</li>']], []], ['html', '</ul>']], []]]
  const out = render(ops, { data: { grupos: [{ id: 'G', itens: ['1', '2'] }] } })
  assert.equal(out, '<ul data-of="a" data-key="G"><li data-of="b" data-key="1">G1</li><li data-of="b" data-key="2">G2</li></ul>')
})

test('caminho inexistente lança erro', () => {
  assert.throws(() => render([['text', 'data.nao.existe']], { data: {} }), /caminho inexistente: data\.nao\.existe/)
  assert.throws(() => render([['text', 'page.x']], { data: {} }), /caminho inexistente/)
  assert.throws(() => render([['show', 'data.a.b', false]], { data: { a: null } }), /caminho inexistente/)
  assert.equal(resolve({ data: { s: 'abc' } }, 'data.s.length'), 3)
})

test('chave repetida, lista inválida e op desconhecida lançam erro', () => {
  assert.throws(() => render(forOps, { data: { itens: [{ id: 1, nome: 'a' }, { id: 1, nome: 'b' }] } }), /data-key repetido/)
  assert.throws(() => render(forOps, { data: { itens: 'abc' } }), /espera uma lista/)
  assert.throws(() => render([['nada']], {}), /op desconhecida/)
})

test('render é determinístico', () => {
  const dados = () => ({ itens: Array.from({ length: 50 }, (_, i) => ({ id: i, nome: `item <${i}>` })) })
  const a = render(forOps, { data: dados() })
  const b = render(forOps, { data: structuredClone(dados()) })
  assert.equal(a, b)
  assert.equal(Buffer.compare(Buffer.from(a), Buffer.from(render(forOps, { data: dados() }))), 0)
})

test('state e shared ficam para o cliente: diretivas intactas, blocos sem instâncias', () => {
  const ops = [
    ['html', '<p data-text="state.n"'], ['show', 'shared.ui.aberto', false], ['attr', 'title', 'state.t'],
    ['class', 'base', [['on', 'state.on', false], ['dado', 'data.ok', false]]], ['html', '>'], ['text', 'state.n'], ['html', '</p>'],
    ['html', '<template data-tid="t1" data-if="state.x"><b></b></template>'],
    ['if', 't1', 'state.x', false, [['html', '<b>x</b>']], [['html', '<i>y</i>']]],
    ['html', '<template data-tid="t2" data-for="i in shared.carrinho.itens" data-key="i.id"><li></li></template>'],
    ['for', 't2', 'i', 'shared.carrinho.itens', 'i.id', [['html', '<li>'], ['text', 'i.nome'], ['html', '</li>']], [['html', '<li>vazio</li>']]],
  ]
  assert.equal(
    render(ops, { data: { ok: true } }),
    '<p data-text="state.n" class="base dado"></p><template data-tid="t1" data-if="state.x"><b></b></template><template data-tid="t2" data-for="i in shared.carrinho.itens" data-key="i.id"><li></li></template>',
  )
})

test('variável de for sobre data resolve no servidor, mesmo junto de state', () => {
  const ops = [['for', 't1', 'item', 'data.itens', 'item.id', [['html', '<li'], ['class', '', [['sel', 'state.sel', false]]], ['html', '>'], ['text', 'item.nome'], ['html', '</li>']], []]]
  assert.equal(render(ops, { data: { itens: [{ id: 1, nome: 'a' }] } }), '<li data-of="t1" data-key="1">a</li>')
  assert.throws(() => render([['for', 't1', 'item', 'data.itens', 'item.id', [['html', '<li>'], ['text', 'item.falta'], ['html', '</li>']], []]], { data: { itens: [{ id: 1 }] } }), /caminho inexistente: item\.falta/)
  for (const path of ['data.x', 'params.x', 'query.x']) {
    assert.throws(() => render([['text', path]], { data: {}, params: {}, query: {} }), /caminho inexistente/)
  }
})
