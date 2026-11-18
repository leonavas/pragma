import { describe, it, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { browsers, launch, start } from './app.mjs'

const PORT = Number(process.env.DEV_PORT ?? 5282)
let app

before(async () => { app = await start('dev', PORT) })
after(() => app?.stop())

const editar = (arquivo, de, para) => {
  const f = join(app.dir, arquivo)
  const src = readFileSync(f, 'utf8')
  assert.ok(src.includes(de), `${arquivo} não contém ${de}`)
  writeFileSync(f, src.replace(de, para))
}

for (const nome of browsers) {
  describe(`${nome} (pragma dev)`, () => {
    let browser, page

    before(async () => {
      browser = await launch(nome)
      page = await browser.newPage({ baseURL: app.url })
    })
    after(() => browser?.close())

    it('deep link com SSR, loader e diretivas em modo dev', async () => {
      await page.goto('/loja/cafe')
      assert.equal(await page.locator('main h1').textContent(), 'Café torrado 500 g')
      assert.equal(await page.evaluate(() => self.__pragmaDev), true)
      await page.goto('/loja')
      await page.locator('main a[href="/loja/cafe"]').waitFor()
    })

    it('live reload: CSS troca sem recarregar; HTML recarrega', async () => {
      await page.goto('/')
      await page.evaluate(() => { self.__marca = 1 })
      const raio = () => page.evaluate(() => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius)
      const antes = await raio()
      editar('lib/theme.css', 'border-radius: var(--radius-2xl);', 'border-radius: 3px;')
      await page.waitForFunction(() => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius === '3px')
      assert.equal(await page.evaluate(() => self.__marca), 1)
      editar('lib/theme.css', 'border-radius: 3px;', 'border-radius: var(--radius-2xl);')
      await page.waitForFunction(r => getComputedStyle(document.querySelector('.card')).borderTopLeftRadius === r, antes)

      editar('routes/index.html', '>Painel</h1>', '>Painel ao vivo</h1>')
      await page.waitForFunction(() => document.querySelector('main h1').textContent === 'Painel ao vivo')
      assert.equal(await page.evaluate(() => self.__marca), undefined)
      editar('routes/index.html', '>Painel ao vivo</h1>', '>Painel</h1>')
      await page.waitForFunction(() => document.querySelector('main h1').textContent === 'Painel')
    })
  })
}
