import { test, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { start } from './server.mjs'

const tools = process.env.PRAGMA_TOOLS ?? '/tmp/pragma-tools-runtime-cliente'
const { chromium } = createRequire(tools + '/')('playwright')

let server, browser

before(async () => {
  server = await start()
  browser = await chromium.launch()
})
after(async () => {
  await browser?.close()
  server?.close()
})

async function open(path) {
  const ctx = await browser.newContext()
  const page = await ctx.newPage()
  page.errors = []
  page.requests = []
  page.on('pageerror', e => page.errors.push(e.message))
  page.on('request', r => page.requests.push(new URL(r.url()).pathname))
  await page.goto(server.url + path)
  await page.waitForFunction(() => document.querySelector('#qtd')?.textContent !== '')
  return page
}

const text = (page, sel) => page.locator(sel).textContent()
const tick = page => page.evaluate(() => new Promise(r => setTimeout(r)))

test('data-text, data-attr, data-class, data-show', async () => {
  const p = await open('/diretivas')
  assert.equal(await text(p, '#nome'), 'Ana')
  const link = await p.evaluate(() => {
    const a = document.querySelector('#link')
    return [a.getAttribute('href'), a.getAttribute('aria-disabled'), a.hasAttribute('title'), a.getAttribute('data-x')]
  })
  assert.deepEqual(link, ['/x', null, false, ''])
  assert.equal(await p.getAttribute('#cls', 'class'), 'base ativo')
  assert.equal(await p.isVisible('#show'), true)
  assert.equal(await p.isVisible('#hide'), false)

  await p.evaluate(() => __ctx.update(s => { s.ativo = false; s.titulo = 'oi'; s.desabilitado = true; s.nome = '<b>x</b>' }))
  assert.equal(await text(p, '#nome'), '<b>x</b>')
  assert.equal(await p.locator('#nome b').count(), 0)
  const link2 = await p.evaluate(() => {
    const a = document.querySelector('#link')
    return [a.getAttribute('aria-disabled'), a.getAttribute('title'), a.hasAttribute('data-x')]
  })
  assert.deepEqual(link2, ['', 'oi', false])
  assert.equal(await p.getAttribute('#cls', 'class'), 'base inativo')
  assert.equal(await p.isVisible('#show'), false)
  assert.equal(await p.isVisible('#hide'), true)
  assert.deepEqual(p.errors, [])
})

test('data-if/data-else, data-with e escrita só quando muda', async () => {
  const p = await open('/diretivas')
  assert.equal(await p.locator('#sim').count(), 1)
  assert.equal(await p.locator('#nao').count(), 0)
  assert.equal(await p.getAttribute('#sim', 'data-of'), 't1')
  await p.click('#alternar')
  assert.equal(await p.locator('#sim').count(), 0)
  assert.equal(await p.getAttribute('#nao', 'data-of'), 't1')
  assert.equal(await text(p, '#w'), 'Bia')
  const writes = await p.evaluate(async () => {
    const seen = []
    const mo = new MutationObserver(rs => seen.push(...rs))
    mo.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true })
    __ctx.update(() => {})
    await new Promise(r => setTimeout(r))
    mo.disconnect()
    return seen.length
  })
  assert.equal(writes, 0)
})

test('data-for/data-empty aninhados e diff por chave preservando nós', async () => {
  const p = await open('/diretivas')
  const items = () => p.$$eval('#lista > li[data-of="t2"][data-key]', ls => ls.map(l => [l.dataset.key, l.querySelector('.n').textContent, l.querySelectorAll('.ok').length, [...l.querySelectorAll('.tag')].map(t => t.textContent).join(), l.querySelectorAll('.sem').length]))
  assert.deepEqual(await items(), [['1', 'um', 1, 'a,b', 0], ['2', 'dois', 0, '', 1]])

  await p.evaluate(() => {
    self.__li = [...document.querySelectorAll('#lista > li[data-of]')]
    self.__tag = document.querySelector('.tag')
    __ctx.update(s => {
      s.itens.unshift({ id: 3, nome: 'tres', ok: true, tags: [{ t: 'z' }] })
      s.itens.reverse()
      s.itens[1].tags.unshift({ t: 'c' })
      s.itens[1].nome = 'UM'
    })
  })
  assert.deepEqual(await items(), [['2', 'dois', 0, '', 1], ['1', 'UM', 1, 'c,a,b', 0], ['3', 'tres', 1, 'z', 0]])
  const same = await p.evaluate(() => {
    const now = [...document.querySelectorAll('#lista > li[data-of]')]
    return [now[0] === __li[1], now[1] === __li[0], document.querySelectorAll('.tag')[1] === __tag]
  })
  assert.deepEqual(same, [true, true, true])

  await p.evaluate(() => __ctx.update(s => { s.itens = [] }))
  assert.deepEqual(await items(), [])
  assert.equal(await p.locator('#vazio').count(), 1)
  await p.evaluate(() => __ctx.update(s => { s.itens = [{ id: 9, nome: 'nove', ok: false, tags: [] }] }))
  assert.equal(await p.locator('#vazio').count(), 0)
  assert.deepEqual(p.errors, [])
})

test('componentes: estado por instância, ação do dono e cleanup ao remover', async () => {
  const p = await open('/diretivas')
  assert.equal(await p.evaluate(() => self.__montados), 2)
  await p.locator('#lista li[data-key="1"] .mais').click()
  await p.locator('#lista li[data-key="1"] .mais').click()
  await p.locator('#lista li[data-key="2"] .mais').click()
  assert.deepEqual(await p.$$eval('.cont', e => e.map(x => x.textContent)), ['2', '1'])
  assert.deepEqual(await p.$$eval('.dono', e => e.map(x => x.textContent)), ['um', 'dois'])
  await p.locator('#lista li[data-key="1"] .rm').click()
  assert.deepEqual(await p.$$eval('.cont', e => e.map(x => x.textContent)), ['1'])
  assert.equal(await p.evaluate(() => self.__desmontados), 1)
})

test('data-model: texto, checkbox, número, select e radio', async () => {
  const p = await open('/diretivas')
  assert.equal(await p.inputValue('#m-text'), 'Ana')
  assert.equal(await p.inputValue('#m-sel'), 'verde')
  assert.equal(await p.isChecked('#r-p'), true)
  await p.fill('#m-text', 'Carla')
  assert.equal(await text(p, '#nome'), 'Carla')
  await p.check('#m-check')
  assert.equal(await text(p, '#eco-aceito'), 'true')
  await p.fill('#m-num', '7')
  assert.equal(await text(p, '#eco-qtd'), '7')
  assert.equal(await p.evaluate(() => typeof __ctx.state.qtd), 'number')
  await p.selectOption('#m-sel', 'roxo')
  assert.equal(await text(p, '#eco-cor'), 'roxo')
  await p.check('#r-g')
  assert.equal(await text(p, '#eco-tam'), 'g')
  await p.evaluate(() => __ctx.update(s => { s.nome = 'Dani'; s.aceito = false; s.tamanho = 'p'; s.cor = 'azul' }))
  assert.equal(await p.inputValue('#m-text'), 'Dani')
  assert.equal(await p.isChecked('#m-check'), false)
  assert.equal(await p.isChecked('#r-p'), true)
  assert.equal(await p.inputValue('#m-sel'), 'azul')
  assert.deepEqual(p.errors, [])
})

test('actions sync e async, ctx.call, submit e ctx.update', async () => {
  const p = await open('/diretivas')
  const reqs = []
  p.on('request', r => r.method() === 'POST' && reqs.push([new URL(r.url()).pathname, r.postData(), r.headers().traceparent?.length]))
  await p.click('#salvar')
  await p.waitForSelector('#salvando', { state: 'visible' })
  await p.waitForSelector('#resposta')
  assert.equal(await text(p, '#resposta'), 'Ana')
  assert.equal(await p.isVisible('#salvando'), false)
  assert.deepEqual(reqs[0], ['/_action/diretivas/salvar', '{"nome":"Ana"}', 55])
  await p.click('#falhar')
  await p.waitForFunction(() => document.querySelector('#erro').textContent === 'Falhou')
  await p.click('#enviar')
  assert.equal(await text(p, '#enviado'), 'true')
  assert.equal(new URL(p.url()).pathname, '/diretivas')
  await p.evaluate(() => setTimeout(() => __ctx.update(s => { s.tick = 42 }), 10))
  await p.waitForFunction(() => document.querySelector('#tick').textContent === '42')
})

test('store persistida reaplica layout e página e sobrevive ao reload', async () => {
  const p = await open('/diretivas')
  assert.equal(await text(p, '#qtd'), '0')
  await p.click('#comprar')
  await p.click('#comprar')
  assert.equal(await text(p, '#qtd'), '2')
  assert.equal(await p.evaluate(() => localStorage.getItem('pragma:carrinho')), '{"itens":[{"id":1},{"id":2}]}')
  await p.reload()
  await p.waitForFunction(() => document.querySelector('#qtd').textContent === '2')
})

test('conteúdo de slot pertence a quem o escreveu (dono léxico)', async () => {
  const p = await open('/slot')
  assert.equal(await text(p, '#caixa-titulo'), 'da caixa')
  assert.equal(await text(p, '#pagina-titulo'), 'da página')
  assert.equal(await text(p, '#pagina-if'), 'da página')
  await p.click('#pagina-btn')
  await p.click('#fundo-btn')
  assert.equal(await p.evaluate(() => __slot.state.cliques), 2)
  assert.equal(await text(p, '#caixa-n'), '5')
  await p.click('#meio-btn')
  await p.click('#caixa-mais')
  assert.equal(await text(p, '#caixa-n'), '7')
  assert.equal(await text(p, '.cont'), '0')
  await p.fill('#pagina-campo', 'oi')
  assert.equal(await text(p, '#campo'), 'oi')
  await p.locator('.item-btn').nth(1).click()
  assert.equal(await text(p, '#escolhido'), 'dois')
  assert.deepEqual(p.errors, [])
})

test('state(ctx) recebe as props da raiz, inclusive data-attr-data-* de quem usa', async () => {
  const p = await open('/slot')
  assert.deepEqual(await p.$$eval('li .n', e => e.map(x => x.textContent)), ['1', '1'])
  assert.deepEqual(await p.$$eval('li .max', e => e.map(x => x.textContent)), ['3', '7'])
  assert.equal(await text(p, '#caixa-n'), '5')
})

test('getters do store são derivados: recalculam e não vão para o localStorage', async () => {
  const p = await open('/slot')
  assert.equal(await text(p, '#qtd'), '10')
  assert.equal(await text(p, '#dobro'), '20')
  await p.click('#pagina-btn')
  assert.equal(await text(p, '#qtd'), '20')
  assert.equal(await text(p, '#dobro'), '40')
  assert.equal(await p.evaluate(() => localStorage.getItem('pragma:conta')), '{"n":2}')
  await p.reload()
  await p.waitForFunction(() => document.querySelector('#qtd').textContent === '20')
  assert.deepEqual(p.errors, [])
})

test('router: aria-current="page" nos links da rota atual', async () => {
  const p = await open('/slot')
  const atuais = () => p.$$eval('a[aria-current="page"]', e => e.map(x => x.id))
  assert.deepEqual(await atuais(), ['nav-slot', 'link-slot'])
  await p.click('#nav-a')
  await p.waitForFunction(() => document.title === 'Página A')
  assert.deepEqual(await atuais(), ['nav-a'])
})

test('overlay.js só é baixado em página com overlay', async () => {
  const p = await open('/diretivas')
  await tick(p)
  assert.equal(p.requests.some(r => r.endsWith('/overlay.js') && r.startsWith('/core/')), false)
  const o = await open('/overlay')
  await o.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/core/overlay.js')))
})

test('data-attr-checked liga a propriedade e bloco novo com autofocus recebe foco', async () => {
  const p = await open('/slot')
  await p.check('#chk')
  await p.evaluate(() => __slot.update(s => { s.marcado = true }))
  await p.evaluate(() => __slot.update(s => { s.marcado = false }))
  assert.equal(await p.isChecked('#chk'), false)
  await p.evaluate(() => __slot.update(s => { s.editando = true }))
  assert.equal(await p.evaluate(() => document.activeElement.id), 'edit')
})

test('caminho inexistente lança erro em dev', async () => {
  const p = await open('/diretivas')
  await p.evaluate(() => __ctx.update(s => { delete s.nome }))
  await tick(p)
  assert.match(p.errors.join(), /caminho inexistente "state.nome"/)
})

test('loader: skeleton, dados, __pd reaproveitado e params', async () => {
  const p = await open('/produto/7')
  assert.equal(await p.locator('#sk').count(), 1)
  assert.equal(await text(p, '#param'), '7')
  await p.waitForSelector('#nome')
  assert.equal(await text(p, '#nome'), 'Produto 7')
  assert.equal(await p.locator('#sk').count(), 0)
  assert.equal(p.requests.filter(r => r === '/_data/produto/7').length, 1)
})

test('loader: erro vira page.error', async () => {
  const p = await open('/erro')
  await p.waitForSelector('#erro')
  assert.equal(await text(p, '#erro'), 'Deu ruim no load')
  assert.equal(await p.locator('#nome').count(), 0)
})

test('SSR: adota o DOM sem recriar nós e liga o estado', async () => {
  const p = await open('/ssr')
  const r = await p.evaluate(() => ({
    vivos: __antes.every(e => e.isConnected),
    mut: __mut,
    lis: [...document.querySelectorAll('#l li[data-of]')].map(l => l.textContent),
    d: document.querySelectorAll('#d').length,
    cd: document.querySelectorAll('#cd').length,
  }))
  assert.equal(r.vivos, true)
  assert.deepEqual(r.mut, ['childList:c'])
  assert.deepEqual(r.lis, ['um', 'dois'])
  assert.equal(r.d, 1)
  assert.equal(r.cd, 1)
  await p.click('#mais')
  assert.equal(await text(p, '#c'), '1')
  assert.equal(p.requests.some(u => u.startsWith('/_data')), false)
  assert.deepEqual(p.errors, [])
})

test('router: link troca main e title, desmonta a página e não recarrega', async () => {
  const p = await open('/a')
  await p.waitForFunction(() => self.__ticks > 2)
  await p.evaluate(() => { self.__marca = 1 })
  await p.evaluate(() => document.querySelector('#ir-b').click())
  await p.waitForFunction(() => document.title === 'Página B')
  assert.equal(await text(p, '#titulo'), 'Página B')
  assert.equal(new URL(p.url()).pathname, '/b')
  assert.equal(await p.evaluate(() => self.__marca), 1)
  assert.equal(await p.evaluate(() => self.__limpou), true)
  const t = await p.evaluate(() => self.__ticks)
  await p.waitForTimeout(80)
  assert.equal(await p.evaluate(() => self.__ticks), t)
  assert.equal(await p.evaluate(() => document.activeElement.matches('main[data-page]')), true)
  await p.evaluate(() => document.querySelector('#d').click())
  await p.waitForSelector('#lista li')
  assert.equal(await text(p, '#nome'), 'Ana')
  assert.equal(await p.evaluate(() => self.__marca), 1)
  assert.deepEqual(p.errors, [])
})

test('router: estático vence param e loader na navegação', async () => {
  const p = await open('/a')
  await p.evaluate(() => document.querySelector('#novo').click())
  await p.waitForFunction(() => document.title === 'Novo produto')
  await p.goBack()
  await p.waitForFunction(() => document.title === 'Página A')
  await p.evaluate(() => document.querySelector('#p1').click())
  await p.waitForSelector('#sk')
  await p.waitForSelector('#nome')
  assert.equal(await text(p, '#nome'), 'Produto 1')
  assert.equal(await text(p, '#param'), '1')
})

test('router: prefetch no hover e por viewport', async () => {
  const p = await open('/a')
  await p.evaluate(() => document.querySelector('#p1').dispatchEvent(new PointerEvent('pointerenter')))
  await p.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.includes('/produto/%5Bid%5D.html') || e.name.includes('/produto/[id].html')))
  const b = await open('/b')
  await b.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/assets/ssr.js')))
  await b.waitForFunction(() => performance.getEntriesByType('resource').some(e => e.name.endsWith('/diretivas.html')))
  assert.equal(b.requests.includes('/ssr'), false)
  assert.equal(b.requests.includes('/volta'), false)
  const before = b.requests.filter(r => r === '/diretivas.html').length
  await b.evaluate(() => document.querySelector('#d').click())
  await b.waitForSelector('#lista li')
  assert.equal(b.requests.filter(r => r === '/diretivas.html').length, before)
})

test('router: voltar e avançar restauram o scroll', async () => {
  const p = await open('/a')
  await p.evaluate(() => scrollTo(0, 1200))
  await p.evaluate(() => document.querySelector('#ir-b').click())
  await p.waitForFunction(() => document.title === 'Página B')
  assert.equal(await p.evaluate(() => scrollY), 0)
  await p.evaluate(() => scrollTo(0, 700))
  await p.goBack()
  await p.waitForFunction(() => document.title === 'Página A')
  await p.waitForFunction(() => scrollY === 1200)
  await p.goForward()
  await p.waitForFunction(() => document.title === 'Página B')
  await p.waitForFunction(() => scrollY === 700)
  assert.deepEqual(p.errors, [])
})

test('router: popstate com overlay aberto é consumido pelo overlay', async () => {
  const p = await open('/overlay')
  await p.evaluate(() => { self.__marca = 1 })
  await p.click('#abrir')
  assert.equal(await p.evaluate(() => document.querySelector('#m').open), true)
  await p.evaluate(() => history.back())
  await p.waitForFunction(() => !document.querySelector('#m').open)
  await p.waitForTimeout(50)
  assert.equal(new URL(p.url()).pathname, '/overlay')
  assert.equal(await p.evaluate(() => self.__marca), 1)
  assert.equal(await text(p, '#titulo'), 'Overlay')
})

test('desempenho: 1000 itens com diff por chave', async () => {
  const p = await open('/lista')
  const r = await p.evaluate(async () => {
    const flush = () => new Promise(r => queueMicrotask(r))
    const time = async fn => {
      const t = performance.now()
      __ctx.update(fn)
      await flush()
      return performance.now() - t
    }
    const antes = [...document.querySelectorAll('#l li')]
    const itens = __ctx.state.itens
    const out = {}
    out.vazia = await time(s => { s.itens = [] })
    out.criar1000 = await time(s => { s.itens = itens })
    const criados = [...document.querySelectorAll('#l li')]
    const um = []
    for (let i = 0; i < 20; i++) um.push(await time(s => { s.itens[500].nome = 'mudou ' + i; s.itens[500].feito = i % 2 === 0 }))
    out.mudar1 = um.sort((a, b) => a - b)[10]
    out.trocar2 = await time(s => { const x = s.itens[1]; s.itens[1] = s.itens[998]; s.itens[998] = x })
    out.remover1 = await time(s => { s.itens.splice(10, 1) })
    const depois = [...document.querySelectorAll('#l li')]
    out.preservou = depois.every(li => criados.includes(li))
    out.n = depois.length
    out.texto = document.querySelectorAll('#l li')[499].textContent
    out.ssrAntes = antes.length
    return out
  })
  assert.equal(r.preservou, true)
  assert.equal(r.n, 999)
  assert.equal(r.texto, 'mudou 19')
  console.log(`desempenho (ms): criar 1000 = ${r.criar1000.toFixed(2)}, mudar 1 de 1000 (mediana) = ${r.mudar1.toFixed(2)}, trocar 2 = ${r.trocar2.toFixed(2)}, remover 1 = ${r.remover1.toFixed(2)}, esvaziar = ${r.vazia.toFixed(2)}`)
})
