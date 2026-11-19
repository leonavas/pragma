import { describe, it, before, after, beforeEach, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { audit, browsers, launch, timers, watchErrors, watchFirstPaint } from './app.mjs'

const PRODUCTS = 9
const NOT_FOUND = /status of 404/
const SERVER_ERROR = /status of 5\d\d/

export function appSuite(mode, app) {
  for (const browserName of browsers) {
    describe(`${browserName} (pragma ${mode})`, () => {
      let browser, ctx, page, errors, allowed

      before(async () => { browser = await launch(browserName) })
      after(() => browser?.close())
      beforeEach(async () => {
        ctx = await browser.newContext({ baseURL: app().url })
        await ctx.addInitScript(timers)
        errors = []
        allowed = []
        page = await openPage()
      })
      afterEach(async () => {
        await ctx.close()
        assert.deepEqual(errors.filter(error => !allowed.some(pattern => pattern.test(error))), [])
      })

      const openPage = async () => {
        const opened = await ctx.newPage()
        watchErrors(opened, errors)
        return opened
      }
      const allowErrors = (...patterns) => allowed.push(...patterns)
      const nav = name => page.getByRole('banner').getByRole('link', { name, exact: true })
      const h1 = () => page.locator('main h1').first().textContent()
      const waitForH1 = text => page.waitForFunction(expected => document.querySelector('main h1')?.textContent === expected, text)
      const mark = () => page.evaluate(() => { self.__marker = 1 })
      const assertNoReload = async () => assert.equal(await page.evaluate(() => self.__marker), 1)
      const cartBadge = (target = page) => target.getByRole('banner').getByRole('link', { name: /Cart/ }).locator('span')
      const productLinks = () => page.locator('main a[href^="/shop/"]')
      const productCount = count => page.waitForFunction(expected => document.querySelectorAll('main a[href^="/shop/"]').length === expected, count)
      const taskCount = count => page.waitForFunction(expected => document.querySelectorAll('main ul li').length === expected, count)
      const activeInside = selector => page.evaluate(sel => !!document.activeElement?.closest(sel), selector)
      const activeText = () => page.evaluate(() => document.activeElement?.textContent.trim())
      const waitForFocus = text => page.waitForFunction(expected => document.activeElement?.textContent.trim() === expected, text)
      const waitForUnlock = () => page.waitForFunction(() => !document.documentElement.hasAttribute('data-scroll-lock'))
      const focusOutside = selector => page.evaluate(sel => {
        const active = document.activeElement
        return !!active && active !== document.body && !active.closest(sel)
      }, selector)
      const scrollLocked = () => page.evaluate(() => document.documentElement.hasAttribute('data-scroll-lock'))
      const storage = key => page.evaluate(name => JSON.parse(localStorage.getItem(name)), key)
      const addFirstProduct = async () => {
        await productLinks().first().waitFor()
        await page.locator('main li').first().getByRole('button', { name: 'Add' }).click()
      }

      describe('navigation', () => {
        it('client router: no reload, title, aria-current, back/forward, focus on main', async () => {
          await page.goto('/')
          await mark()
          assert.equal(await nav('Dashboard').getAttribute('aria-current'), 'page')
          await nav('Tasks').click()
          await page.waitForFunction(() => document.title === 'Tasks · Minimart')
          assert.equal(await h1(), 'Tasks')
          assert.equal(await nav('Tasks').getAttribute('aria-current'), 'page')
          assert.equal(await nav('Dashboard').getAttribute('aria-current'), null)
          assert.equal(await page.evaluate(() => document.activeElement === document.querySelector('main')), true)
          await nav('Shop').click()
          await productLinks().first().waitFor()
          assert.equal(await page.title(), 'Shop · Minimart')
          await productLinks().first().click()
          await waitForH1('Ground coffee 500 g')
          assert.equal(new URL(page.url()).pathname, '/shop/coffee')
          assert.equal(await page.title(), 'Product · Minimart')
          assert.equal(await nav('Shop').getAttribute('aria-current'), null)
          await page.getByRole('link', { name: '← Back to the shop' }).click()
          await waitForH1('Shop')
          await page.goBack()
          await waitForH1('Ground coffee 500 g')
          await page.goBack()
          await waitForH1('Shop')
          await page.goBack()
          await waitForH1('Tasks')
          assert.equal(await nav('Tasks').getAttribute('aria-current'), 'page')
          await page.goForward()
          await waitForH1('Shop')
          await page.goForward()
          await waitForH1('Ground coffee 500 g')
          await assertNoReload()
        })

        it('404: direct load, client link to an unknown path, link back home', async () => {
          allowErrors(NOT_FOUND)
          const response = await page.goto('/does-not-exist')
          assert.equal(response.status(), 404)
          assert.equal(await h1(), 'Page not found')
          assert.equal(await page.title(), 'Page not found')
          await page.getByRole('link', { name: 'Back to home' }).click()
          await waitForH1('Dashboard')

          await mark()
          const unknown = page.waitForResponse(res => new URL(res.url()).pathname === '/nowhere' && res.request().isNavigationRequest())
          await page.evaluate(() => {
            const link = Object.assign(document.createElement('a'), { href: '/nowhere', textContent: 'nowhere' })
            document.querySelector('main').append(link)
            link.click()
          })
          assert.equal((await unknown).status(), 404)
          await waitForH1('Page not found')
          assert.equal(await page.evaluate(() => self.__marker), undefined)
        })

        it('param route /shop/:id: client navigation to a known and an unknown id', async () => {
          await page.goto('/shop')
          await mark()
          await page.locator('main a[href="/shop/banana"]').click()
          await waitForH1('Bananas 1 kg')
          assert.match(await page.locator('main').textContent(), /\$7\.40/)
          await page.evaluate(() => {
            const link = Object.assign(document.createElement('a'), { href: '/shop/unknown-id', textContent: 'unknown' })
            document.querySelector('main').append(link)
            link.click()
          })
          await waitForH1('Product not found')
          assert.equal(new URL(page.url()).pathname, '/shop/unknown-id')
          assert.equal(await page.getByRole('button', { name: 'Like' }).count(), 0)
          await assertNoReload()
          await page.goBack()
          await waitForH1('Bananas 1 kg')
          await page.goto('/shop/missing')
          assert.equal(await h1(), 'Product not found')
        })

        it('hover prefetch downloads the route HTML before the click', async () => {
          await page.goto('/')
          const html = page.waitForRequest(request => request.url().endsWith('/tasks/index.html'))
          await nav('Tasks').hover()
          await html
        })
      })

      describe('SSR', () => {
        it('product HTML has content and data before JS; works without JS', async () => {
          const html = await (await fetch(app().url + '/shop/coffee')).text()
          assert.match(html, /<h1[^>]*>Ground coffee 500 g<\/h1>/)
          assert.match(html, /\$32\.90/)
          assert.match(html, /id="pragma-data"/)
          assert.doesNotMatch(html, /class="[^"]*skeleton/)

          const noJs = await browser.newContext({ baseURL: app().url, javaScriptEnabled: false })
          const noJsPage = await noJs.newPage()
          await noJsPage.goto('/shop/coffee')
          assert.equal(await noJsPage.locator('main h1').textContent(), 'Ground coffee 500 g')
          assert.equal(await noJsPage.locator('main').getByText('$32.90').count(), 1)
          await noJsPage.goto('/shop/missing')
          assert.equal(await noJsPage.locator('main h1').textContent(), 'Product not found')
          await noJs.close()
        })

        it('hydrates without flash: same h1 node, no skeleton, no /_data request, then interactive', async () => {
          await ctx.addInitScript(watchFirstPaint)
          const dataRequests = []
          page.on('request', request => request.url().includes('/_data/') && dataRequests.push(request.url()))
          await page.goto('/shop/coffee')
          await page.reload()
          await page.getByRole('button', { name: 'Add' }).click()
          assert.equal(await cartBadge().textContent(), '1')
          const firstPaint = await page.evaluate(() => ({
            sameH1: self.__firstH1 === document.querySelector('main h1'),
            removedH1: self.__removedH1,
            skeletons: self.__skeletons,
            sawNotFound: self.__addedText.includes('Product not found'),
          }))
          assert.deepEqual(firstPaint, { sameH1: true, removedH1: 0, skeletons: 0, sawNotFound: false })
          assert.equal(await h1(), 'Ground coffee 500 g')
          assert.deepEqual(dataRequests, [])
        })
      })

      describe('loader', () => {
        it('skeleton then data on first load and on client navigation', async () => {
          await page.goto('/shop', { waitUntil: 'commit' })
          await page.locator('main .skeleton').first().waitFor()
          assert.equal(await page.locator('main .skeleton').count(), 4)
          assert.equal(await page.locator('main [aria-busy="true"]').getAttribute('aria-label'), 'Loading products')
          await productCount(PRODUCTS)
          assert.equal(await page.locator('main .skeleton').count(), 0)
          assert.equal(await page.getByText('Sold out', { exact: true }).locator('visible=true').count(), 1)

          await nav('Dashboard').click()
          await waitForH1('Dashboard')
          await mark()
          const data = page.waitForResponse(res => new URL(res.url()).pathname === '/_data/shop')
          await nav('Shop').click()
          await page.locator('main .skeleton').first().waitFor()
          assert.equal(await h1(), 'Shop')
          assert.equal((await data).status(), 200)
          await productCount(PRODUCTS)
          assert.equal(await page.locator('main .skeleton').count(), 0)
          await assertNoReload()
        })

        it('query filters: ?category= in the URL refetches, aria-current, unknown category, search', async () => {
          await page.goto('/shop?category=produce')
          await productCount(3)
          assert.equal(await page.getByRole('link', { name: 'Produce' }).getAttribute('aria-current'), 'true')
          assert.equal(await page.getByRole('link', { name: 'All' }).getAttribute('aria-current'), null)

          await mark()
          const refetch = page.waitForRequest(request => request.url().endsWith('/_data/shop?category=cleaning'))
          await page.getByRole('link', { name: 'Cleaning' }).click()
          await refetch
          await page.waitForURL('**/shop?category=cleaning')
          await page.locator('main a[href="/shop/soap"]').waitFor()
          await productCount(3)
          assert.equal(await page.getByRole('link', { name: 'Cleaning' }).getAttribute('aria-current'), 'true')
          assert.equal(await page.getByRole('link', { name: 'Produce' }).getAttribute('aria-current'), null)
          const search = page.getByRole('searchbox', { name: 'Search products' })
          await search.fill('SOA')
          await productCount(1)
          await search.fill('zzz')
          await page.getByText('No products').waitFor()
          await search.fill('')
          await productCount(3)
          await page.goBack()
          await page.waitForURL('**/shop?category=produce')
          await page.locator('main a[href="/shop/apple"]').waitFor()
          await productCount(3)
          assert.equal(await page.getByRole('link', { name: 'Produce' }).getAttribute('aria-current'), 'true')
          await assertNoReload()

          await page.goto('/shop?category=bogus')
          await productCount(PRODUCTS)
          assert.equal(await page.getByRole('link', { name: 'All' }).getAttribute('aria-current'), 'true')
        })

        it('error state with ?fail=1 and recovery with Try again', async () => {
          allowErrors(SERVER_ERROR, /loader failed/)
          await page.goto('/shop?fail=1')
          await page.getByRole('alert').getByText('Could not load the products.').waitFor()
          assert.equal(await page.locator('main .skeleton').count(), 0)
          assert.equal(await productLinks().count(), 0)
          await page.getByRole('link', { name: 'Try again' }).click()
          await productCount(PRODUCTS)
          assert.equal(await page.getByRole('alert').count(), 0)
          assert.equal(new URL(page.url()).search, '')
        })

        it('leaving before the data arrives logs no loader error', async () => {
          for (let i = 0; i < 3; i++) {
            await page.goto('/shop', { waitUntil: 'commit' })
            await page.locator('main h1').waitFor()
            await page.goto('/tasks')
            await waitForH1('Tasks')
          }
        })
      })

      describe('stores', () => {
        it('cart: quantity, stock, badge, summary and emptying with confirmation', async () => {
          await page.goto('/shop')
          const lettuce = page.locator('main li').filter({ has: page.locator('a[href="/shop/lettuce"]') })
          await lettuce.waitFor()
          const increase = lettuce.getByRole('button', { name: 'Increase quantity' })
          assert.equal(await lettuce.getByRole('button', { name: 'Decrease quantity' }).isDisabled(), true)
          await increase.click()
          await increase.click()
          assert.equal(await increase.isDisabled(), true)
          await lettuce.getByRole('button', { name: 'Add' }).click()
          assert.equal(await cartBadge().textContent(), '3')
          assert.equal(await lettuce.locator('output').textContent(), '1')
          const summary = page.getByRole('complementary', { name: 'Cart' })
          assert.equal(await summary.getByRole('button', { name: 'Increase' }).isDisabled(), true)
          await summary.getByRole('button', { name: 'Decrease' }).click()
          assert.equal(await cartBadge().textContent(), '2')
          assert.match(await summary.textContent(), /\$9\.00/)
          assert.equal(await page.locator('main li').filter({ has: page.locator('a[href="/shop/detergent"]') }).getByRole('button', { name: 'Add' }).isDisabled(), true)

          await page.locator('main li').filter({ has: page.locator('a[href="/shop/rice"]') }).getByRole('button', { name: 'Add' }).click()
          assert.equal(await summary.getByRole('listitem').count(), 2)
          await summary.getByRole('listitem').filter({ hasText: 'Brown rice' }).getByRole('button', { name: 'Remove' }).click()
          assert.equal(await summary.getByRole('listitem').count(), 1)
          assert.equal(await cartBadge().textContent(), '2')

          await summary.getByRole('button', { name: 'Empty cart' }).click()
          const alert = page.getByRole('alertdialog', { name: 'Empty the cart?' })
          await alert.getByRole('button', { name: 'Cancel' }).click()
          assert.equal(await cartBadge().textContent(), '2')
          await summary.getByRole('button', { name: 'Empty cart' }).click()
          await alert.getByRole('button', { name: 'Empty cart' }).click()
          await cartBadge().filter({ hasText: /^0$/ }).waitFor()
          await summary.getByText('Your cart is empty.').waitFor()
          assert.equal(await summary.getByRole('button', { name: 'Empty cart' }).isDisabled(), true)
          assert.deepEqual(await storage('pragma:cart'), { items: [] })
        })

        it('cart and tasks persist across navigation and reload in localStorage, only the persist fields', async () => {
          await page.goto('/shop')
          await addFirstProduct()
          await nav('Tasks').click()
          await page.getByRole('textbox', { name: 'New task' }).fill('Persist me')
          await page.getByRole('textbox', { name: 'New task' }).press('Enter')
          await nav('Dashboard').click()
          await waitForH1('Dashboard')
          assert.equal(await cartBadge().textContent(), '1')

          const cart = await storage('pragma:cart')
          assert.deepEqual(Object.keys(cart), ['items'])
          assert.deepEqual(cart.items.map(({ id, quantity }) => ({ id, quantity })), [{ id: 'coffee', quantity: 1 }])
          assert.deepEqual(Object.keys(cart.items[0]).sort(), ['id', 'name', 'price', 'quantity', 'stock'])
          const tasks = await storage('pragma:tasks')
          assert.deepEqual(Object.keys(tasks), ['items'])
          assert.deepEqual(tasks.items.map(task => task.title), ['Buy coffee', 'Pay bills', 'Call the pharmacy', 'Persist me'])

          await page.reload()
          assert.equal(await cartBadge().textContent(), '1')
          await page.goto('/tasks')
          await page.getByText('Persist me').waitFor()
          assert.equal(await page.locator('[data-c="indicator"] p').first().textContent(), '1 of 4')
          await page.goto('/shop')
          await page.getByRole('complementary', { name: 'Cart' }).getByText('Ground coffee 500 g').waitFor()
        })

        it('stores sync across tabs through the storage event', async () => {
          await page.goto('/')
          const other = await openPage()
          await other.goto('/shop')
          await other.locator('main li').first().getByRole('button', { name: 'Add' }).click()
          assert.equal(await cartBadge(other).textContent(), '1')
          await cartBadge().filter({ hasText: /^1$/ }).waitFor()
          await page.locator('[data-c="indicator"]').filter({ hasText: 'Items in cart' }).getByText('1', { exact: true }).waitFor()
        })
      })

      describe('blocks and components', () => {
        it('Dashboard: indicators reflect stores; counters with limits; tabs with keyboard', async () => {
          await page.goto('/shop')
          await addFirstProduct()
          await nav('Dashboard').click()
          const indicatorValue = title => page.locator('[data-c="indicator"]').filter({ has: page.getByRole('heading', { name: title }) }).locator('p').first()
          await page.getByRole('heading', { name: 'Items in cart' }).waitFor()
          assert.equal(await indicatorValue('Items in cart').textContent(), '1')
          assert.match(await indicatorValue('Cart total').textContent(), /\$32\.90/)
          assert.equal(await indicatorValue('Pending tasks').textContent(), '2')

          const group = name => page.getByRole('group', { name, exact: true })
          const fives = group('Fives')
          assert.equal(await fives.locator('output').textContent(), '10')
          for (let i = 0; i < 8; i++) await fives.getByRole('button', { name: 'Increase Fives' }).click()
          assert.equal(await fives.locator('output').textContent(), '50')
          assert.equal(await fives.getByRole('button', { name: 'Increase Fives' }).isDisabled(), true)
          await group('Clicks').getByRole('button', { name: 'Increase Clicks' }).click()
          assert.equal(await group('Clicks').locator('output').textContent(), '1')
          await group('Clicks').getByRole('button', { name: 'Decrease Clicks' }).click()
          await group('Clicks').getByRole('button', { name: 'Decrease Clicks' }).click()
          assert.equal(await group('Clicks').locator('output').textContent(), '-1')
          for (let i = 0; i < 10; i++) await fives.getByRole('button', { name: 'Decrease Fives' }).click()
          assert.equal(await fives.locator('output').textContent(), '0')
          assert.equal(await fives.getByRole('button', { name: 'Decrease Fives' }).isDisabled(), true)

          await page.getByRole('tab', { name: 'About' }).focus()
          await page.keyboard.press('ArrowRight')
          await page.keyboard.press('ArrowRight')
          assert.equal(await activeText(), 'Counter')
          assert.equal(await page.getByRole('tab', { name: 'Counter' }).getAttribute('aria-selected'), 'true')
          assert.equal(await page.getByRole('tab', { name: 'Counter' }).getAttribute('tabindex'), '0')
          assert.equal(await page.getByRole('tab', { name: 'About' }).getAttribute('tabindex'), '-1')
          assert.equal(await group('In tab').locator('output').textContent(), '3')
          await group('In tab').getByRole('button', { name: 'Increase In tab' }).click()
          assert.equal(await group('In tab').locator('output').textContent(), '6')
          assert.equal(await group('Clicks').locator('output').textContent(), '-1')
          await page.getByRole('tab', { name: 'Counter' }).focus()
          await page.keyboard.press('ArrowRight')
          assert.equal(await page.getByRole('tab', { name: 'About' }).getAttribute('aria-selected'), 'true')
          await page.keyboard.press('ArrowLeft')
          await page.keyboard.press('ArrowLeft')
          assert.equal(await page.getByRole('tab', { name: 'Shortcuts' }).getAttribute('aria-selected'), 'true')
          await page.getByRole('tab', { name: 'Counter' }).click()
          assert.equal(await group('In tab').locator('output').textContent(), '6')
        })

        it('props, conditional props and slots (default and named)', async () => {
          await page.goto('/')
          const indicator = title => page.locator('[data-c="indicator"]').filter({ has: page.getByRole('heading', { name: title }) })
          assert.equal(await indicator('Pending tasks').locator('p').count(), 2)
          assert.equal(await indicator('Pending tasks').locator('p').last().textContent(), 'See Tasks')
          assert.equal(await indicator('Items in cart').locator('p').count(), 1)
          assert.equal(await indicator('Cart total').locator('p').last().textContent(), 'Shop prices')
          assert.equal(await indicator('Items in cart').getAttribute('data-tone'), 'warning')
          assert.equal(await indicator('Pending tasks').getAttribute('data-tone'), 'neutral')

          const panel = name => page.getByRole('tabpanel', { name, includeHidden: true })
          assert.match(await panel('About').textContent(), /native HTML, CSS and JS/)
          assert.equal(await panel('About').isVisible(), true)
          assert.equal(await panel('Shortcuts').isVisible(), false)
          assert.equal(await panel('Shortcuts').locator('li').count(), 3)
          await page.getByRole('tab', { name: 'Shortcuts' }).click()
          assert.equal(await panel('Shortcuts').isVisible(), true)
          assert.equal(await panel('About').isVisible(), false)

          await page.getByRole('button', { name: 'About the app' }).click()
          const modal = page.getByRole('dialog', { name: 'About Minimart' })
          await modal.waitFor()
          assert.equal(await modal.getByRole('heading', { level: 2 }).textContent(), 'About Minimart')
          assert.match(await modal.locator('div > p').textContent(), /every page is ready-made HTML/)
          assert.equal(await modal.locator('footer').getByRole('button', { name: 'Close', exact: true }).count(), 1)
          await modal.locator('footer').getByRole('button', { name: 'Close', exact: true }).click()
          await modal.waitFor({ state: 'hidden' })

          await nav('Tasks').click()
          await page.getByRole('heading', { name: 'Completed' }).waitFor()
          await page.getByRole('button', { name: 'Actions' }).click()
          await page.getByRole('button', { name: 'Complete all' }).waitFor()
          await page.getByRole('button', { name: 'Complete all' }).click()
          await page.locator('[data-c="indicator"][data-tone="ok"]').waitFor()
          await nav('Dashboard').click()
          await indicator('Pending tasks').and(page.locator('[data-tone="warning"]')).waitFor()
        })

        it('tasks: add, validate, complete, edit (<if>/<else>), remove, search, filter, clear', async () => {
          await page.goto('/tasks')
          const field = page.getByRole('textbox', { name: 'New task' })
          const items = page.locator('main ul li')
          const error = page.locator('#task-error')
          await field.press('Enter')
          assert.equal(await error.textContent(), 'Use at least 3 characters.')
          assert.equal(await field.getAttribute('aria-invalid'), 'true')
          await field.fill('ab')
          await field.press('Enter')
          assert.equal(await error.textContent(), 'Use at least 3 characters.')
          await field.fill('x'.repeat(81))
          await field.press('Enter')
          assert.equal(await error.textContent(), 'Use at most 80 characters.')
          await field.fill('buy COFFEE')
          await field.press('Enter')
          assert.equal(await error.textContent(), 'This task already exists.')
          await field.fill('  Buy bread  ')
          await page.getByRole('button', { name: 'Add', exact: true }).click()
          assert.equal(await error.textContent(), '')
          assert.equal(await field.getAttribute('aria-invalid'), 'false')
          assert.equal(await field.inputValue(), '')
          assert.equal(await page.evaluate(() => document.activeElement.id), 'new-task')
          await taskCount(4)
          await page.getByText('Buy bread', { exact: true }).waitFor()

          await page.getByRole('checkbox', { name: 'Buy bread' }).check()
          assert.equal(await page.locator('[data-c="indicator"] p').first().textContent(), '2 of 4')
          assert.match(await items.filter({ hasText: 'Buy bread' }).locator('span').getAttribute('class'), /line-through/)

          await page.getByText('Call the pharmacy').dblclick()
          const editField = page.getByRole('textbox', { name: 'Edit task' })
          assert.equal(await page.evaluate(() => document.activeElement.getAttribute('aria-label')), 'Edit task')
          assert.equal(await editField.inputValue(), 'Call the pharmacy')
          await editField.fill('Call the doctor')
          await editField.press('Enter')
          await page.getByText('Call the doctor').waitFor()
          assert.equal(await editField.count(), 0)
          await page.getByText('Call the doctor').dblclick()
          await editField.fill('Something else')
          await editField.press('Escape')
          await page.getByText('Call the doctor').waitFor()
          await page.getByText('Call the doctor').dblclick()
          await editField.fill('Pay bills')
          await editField.press('Enter')
          await page.getByText('Call the doctor').waitFor()
          await page.getByText('Call the doctor').dblclick()
          await editField.fill('Call the dentist')
          await field.click()
          await page.getByText('Call the dentist').waitFor()

          await page.getByRole('button', { name: 'Remove Call the dentist' }).click()
          await taskCount(3)
          await page.getByRole('searchbox', { name: 'Search tasks' }).fill('PAY')
          await taskCount(1)
          await page.getByRole('searchbox', { name: 'Search tasks' }).fill('')
          await taskCount(3)

          await page.getByRole('link', { name: 'Pending' }).click()
          await page.waitForURL('**/tasks?filter=pending')
          await taskCount(1)
          await page.getByRole('link', { name: 'Completed' }).click()
          await page.waitForURL('**/tasks?filter=completed')
          await taskCount(2)
          await page.goBack()
          await page.waitForURL('**/tasks?filter=pending')
          await taskCount(1)
          await page.reload()
          await taskCount(1)
          assert.equal(await page.getByRole('link', { name: 'Pending' }).getAttribute('aria-current'), 'true')
          await page.getByRole('checkbox').click()
          await items.getByText('No tasks').waitFor()

          await page.getByRole('link', { name: 'All' }).click()
          await page.waitForURL('**/tasks?filter=all')
          await page.getByRole('button', { name: 'Actions' }).click()
          await page.getByRole('button', { name: 'Clear completed…' }).click()
          const alert = page.getByRole('alertdialog', { name: 'Clear completed tasks?' })
          await alert.getByRole('button', { name: 'Cancel' }).click()
          await taskCount(3)
          await page.getByRole('button', { name: 'Actions' }).click()
          await page.getByRole('button', { name: 'Clear completed…' }).click()
          await alert.getByRole('button', { name: 'Clear' }).click()
          await items.getByText('No tasks').waitFor()
          assert.deepEqual(await storage('pragma:tasks'), { items: [] })
        })

        it('<for> keys keep DOM identity on add, remove, toggle and reorder', async () => {
          await page.goto('/tasks')
          await taskCount(3)
          const tag = () => page.evaluate(() => { for (const li of document.querySelectorAll('main ul li')) li.__key = li.querySelector('span').textContent })
          const identity = () => page.evaluate(() => [...document.querySelectorAll('main ul li')].map(li => ({ title: li.querySelector('span')?.textContent, kept: li.__key === li.querySelector('span')?.textContent })))
          await tag()

          await page.getByRole('textbox', { name: 'New task' }).fill('Water plants')
          await page.getByRole('textbox', { name: 'New task' }).press('Enter')
          await taskCount(4)
          assert.deepEqual(await identity(), [
            { title: 'Buy coffee', kept: true },
            { title: 'Pay bills', kept: true },
            { title: 'Call the pharmacy', kept: true },
            { title: 'Water plants', kept: false },
          ])

          await tag()
          await page.getByRole('button', { name: 'Remove Pay bills' }).click()
          await taskCount(3)
          await page.getByRole('checkbox', { name: 'Buy coffee' }).check()
          assert.deepEqual(await identity(), [
            { title: 'Buy coffee', kept: true },
            { title: 'Call the pharmacy', kept: true },
            { title: 'Water plants', kept: true },
          ])

          const other = await openPage()
          await other.goto('/tasks')
          await other.evaluate(() => {
            const saved = JSON.parse(localStorage.getItem('pragma:tasks'))
            saved.items.reverse()
            localStorage.setItem('pragma:tasks', JSON.stringify(saved))
          })
          await page.waitForFunction(() => document.querySelector('main ul li span').textContent === 'Water plants')
          assert.deepEqual(await identity(), [
            { title: 'Water plants', kept: true },
            { title: 'Call the pharmacy', kept: true },
            { title: 'Buy coffee', kept: true },
          ])
          assert.equal(await page.getByRole('checkbox', { name: 'Buy coffee' }).isChecked(), true)
        })
      })

      describe('overlays', () => {
        it('modal: focus inside and trapped, Back closes without navigating, Esc and backdrop close, focus returns', async () => {
          await page.goto('/')
          await mark()
          const aboutButton = page.getByRole('button', { name: 'About the app' })
          const modal = page.getByRole('dialog', { name: 'About Minimart' })
          await aboutButton.click()
          await modal.waitFor()
          assert.equal(await scrollLocked(), true)
          assert.equal(await activeInside('dialog#about'), true)
          for (let i = 0; i < 6; i++) {
            await page.keyboard.press(i < 4 ? 'Tab' : 'Shift+Tab')
            assert.equal(await focusOutside('dialog#about'), false, `focus reached the page behind the modal after key ${i + 1}`)
          }
          await page.keyboard.press('Shift+Tab')
          await page.keyboard.press('Tab')
          await page.keyboard.press('Tab')
          await page.keyboard.press('Tab')
          assert.equal(await focusOutside('dialog#about'), false)

          await page.goBack()
          await modal.waitFor({ state: 'hidden' })
          assert.equal(new URL(page.url()).pathname, '/')
          await waitForUnlock()
          await waitForFocus('About the app')

          await aboutButton.click()
          await modal.waitFor()
          await page.keyboard.press('Escape')
          await modal.waitFor({ state: 'hidden' })
          await waitForFocus('About the app')

          await aboutButton.click()
          await modal.waitFor()
          await page.mouse.click(2, 2)
          await modal.waitFor({ state: 'hidden' })
          await waitForUnlock()
          await waitForFocus('About the app')

          await aboutButton.click()
          await modal.getByRole('button', { name: 'Close', exact: true }).first().click()
          await modal.waitFor({ state: 'hidden' })
          await assertNoReload()

          await nav('Tasks').click()
          await waitForH1('Tasks')
          await page.goBack()
          await waitForH1('Dashboard')
          await assertNoReload()
        })

        it('modal locks scroll without shifting the layout', async () => {
          await page.goto('/')
          const bodyWidth = () => page.evaluate(() => document.body.getBoundingClientRect().width)
          const widthBefore = await bodyWidth()
          await page.getByRole('button', { name: 'About the app' }).click()
          await page.getByRole('dialog', { name: 'About Minimart' }).waitFor()
          assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).overflow), 'hidden')
          assert.equal(await bodyWidth(), widthBefore)
          await page.keyboard.press('Escape')
          await waitForUnlock()

          await page.setViewportSize({ width: 800, height: 300 })
          await page.evaluate(() => scrollTo(0, 200))
          const scrolledWidth = await bodyWidth()
          await page.evaluate(() => document.querySelector('[data-open="about"]').click())
          await page.getByRole('dialog', { name: 'About Minimart' }).waitFor()
          await page.mouse.wheel(0, 400)
          await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))))
          assert.equal(await page.evaluate(() => scrollY), 200)
          assert.equal(await bodyWidth(), scrolledWidth)
          await page.getByRole('dialog', { name: 'About Minimart' }).getByRole('button', { name: 'Close', exact: true }).last().click()
          assert.equal(await page.evaluate(() => scrollY), 200)
        })

        it('alert: autofocus on the safe button, Esc cancels, backdrop does not close, Back closes, focus returns', async () => {
          await page.goto('/shop')
          await addFirstProduct()
          const summary = page.getByRole('complementary', { name: 'Cart' })
          const emptyButton = summary.getByRole('button', { name: 'Empty cart' })
          const alert = page.getByRole('alertdialog', { name: 'Empty the cart?' })
          await mark()

          await emptyButton.click()
          await alert.waitFor()
          assert.equal(await activeText(), 'Cancel')
          assert.equal(await scrollLocked(), true)
          await page.keyboard.press('Escape')
          await alert.waitFor({ state: 'hidden' })
          assert.equal(await cartBadge().textContent(), '1')
          await waitForFocus('Empty cart')

          await emptyButton.click()
          await alert.waitFor()
          await page.mouse.click(2, 2)
          assert.equal(await alert.isVisible(), true)
          await page.goBack()
          await alert.waitFor({ state: 'hidden' })
          assert.equal(new URL(page.url()).pathname, '/shop')
          assert.equal(await cartBadge().textContent(), '1')
          await waitForUnlock()
          await assertNoReload()
        })

        it('dropdown: opens under the trigger, Esc, outside click and Back close it, history does not pile up', async () => {
          await page.goto('/')
          await nav('Tasks').click()
          await waitForH1('Tasks')
          await mark()
          const trigger = page.getByRole('button', { name: 'Actions' })
          const menu = page.locator('#task-actions')
          const isOpen = () => menu.evaluate(el => el.matches(':popover-open'))

          await trigger.click()
          await page.getByRole('button', { name: 'Complete all' }).waitFor()
          assert.equal(await isOpen(), true)
          assert.equal(await scrollLocked(), false)
          const [menuBox, triggerBox] = [await menu.boundingBox(), await trigger.boundingBox()]
          assert.ok(menuBox.y >= triggerBox.y + triggerBox.height, 'menu is below the trigger')
          await page.keyboard.press('Escape')
          await menu.waitFor({ state: 'hidden' })
          await waitForFocus('Actions')

          await trigger.click()
          await menu.waitFor()
          await page.getByRole('heading', { name: 'Tasks', level: 1 }).click()
          await menu.waitFor({ state: 'hidden' })

          await trigger.click()
          await menu.waitFor()
          await trigger.click()
          await menu.waitFor({ state: 'hidden' })

          await trigger.click()
          await menu.waitFor()
          await page.goBack()
          await menu.waitFor({ state: 'hidden' })
          assert.equal(new URL(page.url()).pathname, '/tasks')

          await trigger.click()
          await page.getByRole('button', { name: 'Clear completed…' }).click()
          const alert = page.getByRole('alertdialog', { name: 'Clear completed tasks?' })
          await alert.waitFor()
          assert.equal(await isOpen(), false)
          await page.keyboard.press('Escape')
          await alert.waitFor({ state: 'hidden' })
          await waitForFocus('Actions')
          await taskCount(3)

          await page.goBack()
          await waitForH1('Dashboard')
          await assertNoReload()
        })
      })

      describe('theme', () => {
        it('toggle persists across reload and applies before JS (no flash)', async () => {
          await page.goto('/')
          const toggle = page.getByRole('button', { name: /theme/ })
          assert.equal(await toggle.textContent(), 'Dark theme')
          assert.equal(await toggle.getAttribute('aria-pressed'), 'false')
          await toggle.click()
          assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'dark')
          assert.equal(await page.evaluate(() => localStorage.getItem('pragma:theme')), 'dark')
          assert.equal(await toggle.textContent(), 'Light theme')
          assert.equal(await toggle.getAttribute('aria-pressed'), 'true')
          const darkBackground = await page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor)
          await page.reload()
          assert.equal(await page.getByRole('button', { name: 'Light theme' }).getAttribute('aria-pressed'), 'true')
          await nav('Tasks').click()
          await waitForH1('Tasks')
          assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'dark')

          allowErrors(/net::ERR_FAILED/)
          await page.route('**/*.js', route => route.abort())
          await page.goto('/shop')
          assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'dark')
          assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor), darkBackground)
          await page.unroute('**/*.js')

          await page.goto('/')
          await page.getByRole('button', { name: 'Light theme' }).click()
          await page.reload()
          assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'light')
          assert.notEqual(await page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor), darkBackground)
        })
      })

      describe('lifecycle', () => {
        it('the clock timer stops when leaving the Dashboard', async () => {
          await page.goto('/')
          const time = await page.locator('main time').textContent()
          assert.match(time, /^\d\d:\d\d:\d\d$/)
          await page.waitForFunction(previous => document.querySelector('main time').textContent !== previous, time)
          assert.equal(await page.evaluate(() => __timers()), 1)
          await nav('Tasks').click()
          await page.waitForFunction(() => document.title.startsWith('Tasks'))
          assert.equal(await page.evaluate(() => __timers()), 0)
          await nav('Dashboard').click()
          await page.locator('main time').waitFor()
          assert.equal(await page.evaluate(() => __timers()), 1)
        })
      })

      describe('server actions', () => {
        it('like via call(): pending state, traceparent, server log, persisted on reload', async () => {
          await page.goto('/shop/banana')
          const likes = page.locator('#likes')
          const likesBefore = Number(await likes.textContent())
          let release
          const gate = new Promise(resolve => { release = resolve })
          await page.route('**/_action/shop/banana/like', async route => { await gate; await route.continue() })
          const action = page.waitForResponse(response => response.url().endsWith('/_action/shop/banana/like'))
          await page.getByRole('button', { name: 'Like' }).click()
          await page.locator('main button:disabled', { hasText: 'Like' }).waitFor()
          release()
          assert.equal((await action).status(), 200)
          assert.equal((await action).request().method(), 'POST')
          assert.ok((await action).request().headers().traceparent)
          await page.waitForFunction(expected => Number(document.querySelector('#likes').textContent) === expected, likesBefore + 1)
          await page.locator('main button:enabled', { hasText: 'Like' }).waitFor()
          await page.unroute('**/_action/shop/banana/like')
          await page.reload()
          assert.equal(Number(await likes.textContent()), likesBefore + 1)
          for (let i = 0; i < 100 && !/product liked/.test(app().log()); i++) await new Promise(resolve => setTimeout(resolve, 20))
          assert.match(app().log(), /product liked/)
        })

        it('like error path: failing action shows the error, keeps the count, recovers', async () => {
          allowErrors(SERVER_ERROR, /failed to like product/)
          await page.goto('/shop/apple')
          const likes = page.locator('#likes')
          const likesBefore = Number(await likes.textContent())
          const status = page.locator('main [role="status"]')
          await page.route('**/_action/shop/apple/like', route => route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: { message: 'boom' } }) }))
          const otel = page.waitForRequest(request => request.url().endsWith('/_otel') && request.postData()?.includes('failed to like product'))
          await page.getByRole('button', { name: 'Like' }).click()
          await status.getByText('Could not save your like right now.').waitFor()
          assert.equal(Number(await likes.textContent()), likesBefore)
          assert.equal(await page.getByRole('button', { name: 'Like' }).isEnabled(), true)
          const record = JSON.parse((await otel).postData()).logs.find(entry => entry.body === 'failed to like product')
          assert.equal(record.severityText, 'ERROR')
          assert.equal(record.attributes['exception.message'], 'boom')

          await page.unroute('**/_action/shop/apple/like')
          await page.getByRole('button', { name: 'Like' }).click()
          await page.waitForFunction(expected => Number(document.querySelector('#likes').textContent) === expected, likesBefore + 1)
          assert.equal(await status.textContent(), '')

          const missing = await fetch(app().url + '/_action/shop/missing/like', { method: 'POST', headers: { 'content-type': 'application/json' }, body: 'null' })
          assert.ok(missing.status >= 400, `status ${missing.status}`)
          assert.equal((await missing.json()).error?.message, 'Product not found.')
        })

        it('client logs go to /_otel', async () => {
          await page.goto('/shop/banana')
          const otel = page.waitForRequest(request => request.url().endsWith('/_otel') && request.postData()?.includes('product added to cart'))
          await page.getByRole('button', { name: 'Add' }).click()
          const batch = JSON.parse((await otel).postData())
          const record = batch.logs.find(entry => entry.body === 'product added to cart')
          assert.equal(record.severityText, 'INFO')
          assert.equal(record.attributes['product.id'], 'banana')
          assert.equal(record.attributes['url.path'], '/shop/banana')
        })
      })

      describe('a11y and console', () => {
        const pages = [
          ['/', 'Dashboard'],
          ['/tasks', 'Tasks'],
          ['/shop', 'Shop'],
          ['/shop?category=cleaning', 'Shop'],
          ['/shop/coffee', 'Ground coffee 500 g'],
          ['/shop/missing', 'Product not found'],
          ['/does-not-exist', 'Page not found'],
        ]
        it('every page: one h1, names on controls, alt, unique ids, zero console errors (direct load)', async () => {
          allowErrors(NOT_FOUND)
          for (const [url, title] of pages) {
            await page.goto(url)
            await waitForH1(title)
            if (/^\/shop(\?|$)/.test(url)) await productLinks().first().waitFor()
            assert.deepEqual(await page.evaluate(audit), [], url)
          }
          assert.deepEqual(errors.filter(error => !NOT_FOUND.test(error)), [])
        })

        it('every page: audit and zero console errors after client navigation, with overlays open', async () => {
          await page.goto('/')
          await page.getByRole('button', { name: 'About the app' }).click()
          await page.getByRole('dialog', { name: 'About Minimart' }).waitFor()
          assert.deepEqual(await page.evaluate(audit), [])
          await page.keyboard.press('Escape')
          await nav('Tasks').click()
          await waitForH1('Tasks')
          assert.deepEqual(await page.evaluate(audit), [])
          await page.getByText('Buy coffee').dblclick()
          assert.deepEqual(await page.evaluate(audit), [])
          await page.keyboard.press('Escape')
          await nav('Shop').click()
          await productCount(PRODUCTS)
          assert.deepEqual(await page.evaluate(audit), [])
          await addFirstProduct()
          await productLinks().first().click()
          await waitForH1('Ground coffee 500 g')
          assert.deepEqual(await page.evaluate(audit), [])
          assert.deepEqual(errors, [])
        })
      })
    })
  }
}
