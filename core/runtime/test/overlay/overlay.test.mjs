import { describe, it, before, after, beforeEach, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { serve } from './server.mjs'

const pw = await import(process.env.PLAYWRIGHT ?? '/tmp/pragma-tools-overlays/node_modules/playwright/index.mjs')

const aberto = id => document.getElementById(id).matches('[open], :popover-open')

for (const nome of ['chromium', 'firefox']) {
  describe(nome, () => {
    let server, browser, page, base

    before(async () => {
      server = await serve()
      base = `http://127.0.0.1:${server.address().port}`
      // Barra de rolagem clássica (o headless esconde) para medir o salto de layout do scroll lock.
      browser = await pw[nome].launch(nome == 'chromium' ? { ignoreDefaultArgs: ['--hide-scrollbars'] } : {})
    })
    after(async () => {
      await browser.close()
      server.close()
    })
    afterEach(() => page.close())
    beforeEach(async () => {
      page = await browser.newPage()
      await page.goto(base + '/inicio.html')
      await page.click('#ir')
      await page.waitForFunction(() => self.t)
      await page.evaluate(() => {
        self.pops = 0
        addEventListener('popstate', () => self.pops++)
      })
    })

    const estado = () => page.evaluate(() => ({
      path: location.pathname,
      navegacoes: self.navegacoes,
      lock: document.documentElement.hasAttribute('data-scroll-lock'),
      foco: document.activeElement?.id,
    }))
    const esperaAberto = (id, v = true) => page.waitForFunction(([id, v]) =>
      document.getElementById(id).matches('[open], :popover-open') == v, [id, v])
    const esperaLock = v => page.waitForFunction(v => document.documentElement.hasAttribute('data-scroll-lock') == v, v)
    const esperaPops = n => page.waitForFunction(n => self.pops == n, n)
    const voltaAoInicio = async () => {
      await page.goBack()
      await page.waitForURL(/inicio\.html$/)
    }

    it('abre e fecha pelo botão, sem navegar e consumindo a entrada', async () => {
      await page.click('#abre-sobre')
      assert.equal(await page.evaluate(aberto, 'sobre'), true)
      assert.equal((await estado()).lock, true)
      await page.click('#ok-sobre')
      await esperaAberto('sobre', false)
      await esperaPops(1)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-sobre' })
      await voltaAoInicio()
    })

    it('Esc fecha o modal', async () => {
      await page.click('#abre-sobre')
      await page.keyboard.press('Escape')
      await esperaAberto('sobre', false)
      await esperaPops(1)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-sobre' })
      await voltaAoInicio()
    })

    it('clique no fundo fecha o modal, mas não o alert', async () => {
      await page.click('#abre-sobre')
      await page.mouse.click(5, 5)
      await esperaAberto('sobre', false)
      await esperaPops(1)
      await page.click('#abre-apagar')
      await page.mouse.click(5, 5)
      await page.waitForTimeout(100)
      assert.equal(await page.evaluate(aberto, 'apagar'), true)
      await page.click('#cancela-apagar')
      await esperaAberto('apagar', false)
      await esperaPops(2)
      assert.equal((await estado()).navegacoes, 0)
      await voltaAoInicio()
    })

    it('voltar fecha o overlay sem mudar a URL', async () => {
      await page.click('#abre-sobre')
      await page.goBack()
      await esperaAberto('sobre', false)
      await esperaLock(false)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-sobre' })
      await voltaAoInicio()
    })

    it('pilha de dois overlays fecha do topo para baixo', async () => {
      await page.click('#abre-sobre')
      await page.click('#abre-detalhes')
      assert.equal(await page.evaluate(aberto, 'detalhes'), true)
      await page.keyboard.press('Escape')
      await esperaAberto('detalhes', false)
      await esperaPops(1)
      assert.equal(await page.evaluate(aberto, 'sobre'), true)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: true, foco: 'abre-detalhes' })
      await page.goBack()
      await esperaAberto('sobre', false)
      await esperaLock(false)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-sobre' })
      await voltaAoInicio()
    })

    it('scroll lock trava e libera', async () => {
      await page.evaluate(() => scrollTo(0, 500))
      await page.evaluate(() => t.open('sobre'))
      const travado = await page.evaluate(() => getComputedStyle(document.documentElement).overflow)
      assert.equal(travado, 'hidden')
      await page.mouse.wheel(0, 800)
      await page.waitForTimeout(100)
      assert.equal(await page.evaluate(() => scrollY), 500)
      await page.click('#ok-sobre')
      await esperaLock(false)
      assert.equal(await page.evaluate(() => scrollY), 500)
      assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).overflow), 'visible')
      await page.click('#abre-mais')
      assert.equal((await estado()).lock, false)
    })

    for (const alto of [true, false]) {
      it(`scroll lock não desloca o layout (${alto ? 'com' : 'sem'} barra de rolagem)`, async () => {
        await page.evaluate(alto => document.querySelector('main').classList.toggle('alto', alto), alto)
        const largura = () => page.evaluate(() => document.body.getBoundingClientRect().width)
        const antes = await largura()
        await page.evaluate(() => t.open('sobre'))
        await esperaLock(true)
        assert.deepEqual(await largura(), antes)
        await page.click('#ok-sobre')
        await esperaLock(false)
        assert.deepEqual(await largura(), antes)
      })
    }

    it('foco volta a quem abriu, também via open()', async () => {
      await page.focus('#fora')
      await page.evaluate(() => t.open('sobre'))
      await page.evaluate(() => t.close('sobre'))
      await esperaAberto('sobre', false)
      await esperaPops(1)
      assert.equal((await estado()).foco, 'fora')
      await voltaAoInicio()
    })

    it('confirm() resolve com o value de cada botão ou cancel', async () => {
      const casos = [
        ['apagar', () => page.click('#confirma-apagar'), 'ok'],
        ['apagar', () => page.click('#cancela-apagar'), 'cancel'],
        ['apagar', () => page.keyboard.press('Escape'), 'cancel'],
        ['apagar', () => page.goBack(), 'cancel'],
        ['apagar', () => page.evaluate(() => t.close('apagar')), 'cancel'],
        ['sobre', () => page.click('#ok-sobre'), 'ok'],
        ['sobre', () => page.click('#x-sobre'), 'cancel'],
        ['sobre', () => page.mouse.click(5, 5), 'cancel'],
      ]
      for (const [id, fecha, esperado] of casos) {
        await page.evaluate(id => { self.r = t.confirm(id) }, id)
        await esperaAberto(id)
        await fecha()
        assert.equal(await page.evaluate(() => self.r), esperado)
      }
      await esperaPops(casos.length)
      assert.equal((await estado()).navegacoes, 0)
      await voltaAoInicio()
    })

    it('dropdown abre junto ao botão, alterna e fecha por item, Esc, fora e voltar', async () => {
      await page.click('#abre-mais')
      await esperaAberto('mais')
      const pos = await page.evaluate(() => {
        const b = document.getElementById('abre-mais').getBoundingClientRect()
        const m = document.getElementById('mais').getBoundingClientRect()
        return Math.round(m.top - b.bottom)
      })
      assert.equal(pos, 4)
      assert.equal((await estado()).lock, false)

      await page.goBack()
      await esperaAberto('mais', false)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-mais' })

      let pops = 1
      for (const fecha of [
        () => page.click('#abre-mais'),
        () => page.keyboard.press('Escape'),
        () => page.click('#fora'),
        () => page.click('#item-copiar'),
      ]) {
        await page.click('#abre-mais')
        await esperaAberto('mais')
        await fecha()
        await esperaAberto('mais', false)
        await esperaPops(++pops)
      }
      assert.equal((await estado()).foco, 'abre-mais')
      assert.equal((await estado()).navegacoes, 0)
      await voltaAoInicio()
    })

    it('item do dropdown abre o alert e o histórico fica consistente', async () => {
      await page.click('#abre-mais')
      await page.click('#item-apagar')
      await esperaAberto('apagar')
      await esperaAberto('mais', false)
      await esperaPops(1)
      await page.click('#confirma-apagar')
      await esperaAberto('apagar', false)
      await esperaPops(2)
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-mais' })
      await voltaAoInicio()
    })

    it('confirm() chamado de dentro do dropdown devolve o foco ao gatilho', async () => {
      await page.click('#abre-mais')
      await page.click('#item-js')
      await esperaAberto('apagar')
      await page.click('#confirma-apagar')
      await esperaPops(2)
      assert.equal(await page.evaluate(() => self.r), 'ok')
      assert.deepEqual(await estado(), { path: '/index.html', navegacoes: 0, lock: false, foco: 'abre-mais' })
      await voltaAoInicio()
    })

    it('fechar pela UI não navega nem acumula entradas', async () => {
      for (let i = 0; i < 3; i++) {
        await page.click('#abre-sobre')
        await page.click('#ok-sobre')
        await esperaAberto('sobre', false)
        await esperaPops(i + 1)
      }
      assert.equal((await estado()).navegacoes, 0)
      await voltaAoInicio()
    })

    it('closeAll() fecha tudo sem mexer no histórico', async () => {
      await page.click('#abre-sobre')
      await page.click('#abre-detalhes')
      const r = await page.evaluate(() => { self.r = t.confirm('detalhes'); return t.closeAll() })
      assert.equal(r, 2)
      assert.equal(await page.evaluate(() => self.r), 'cancel')
      await esperaAberto('sobre', false)
      await esperaAberto('detalhes', false)
      await page.waitForTimeout(100)
      assert.equal(await page.evaluate(() => self.pops), 0)
      assert.equal((await estado()).lock, false)
    })

    it('id inexistente vai para log.error', async () => {
      await page.evaluate(() => t.open('nada'))
      assert.deepEqual(await page.evaluate(() => t.erros), [['overlay não encontrado', { 'pragma.overlay.id': 'nada' }]])
    })
  })
}
