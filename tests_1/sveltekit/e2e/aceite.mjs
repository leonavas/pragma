// Teste de aceite (Playwright). Playwright NÃO é dependência do projeto:
//   mkdir -p /tmp/pw && cd /tmp/pw && npm init -y && npm i playwright
//   cp <projeto>/e2e/aceite.mjs /tmp/pw/ && node /tmp/pw/aceite.mjs http://localhost:5171
// Uso: node test.mjs http://localhost:5171
import { chromium } from 'playwright';
import assert from 'node:assert/strict';

const BASE = process.argv[2] ?? 'http://localhost:5171';
const results = [];
const erros = [];
async function caso(nome, fn) {
	try { await fn(); results.push(['✅', nome]); }
	catch (e) { results.push(['❌', nome, e.message.split('\n').slice(0, 6).join(' | ')]); }
}

let browser;
try { browser = await chromium.launch(); } catch { browser = await chromium.launch({ executablePath: '/usr/bin/chromium' }); }

// instrumenta timers (critério 7) e conta fetches de produtos.json (critério 8)
const INIT = () => {
	const ativos = new Set(); let ticks = 0;
	const si = window.setInterval.bind(window), ci = window.clearInterval.bind(window);
	// só intervalos de 1 s (o do Clock); o cliente do Vite em dev tem um de 30 s
	window.setInterval = (fn, ms, ...a) => { if (ms !== 1000) return si(fn, ms, ...a); const id = si(() => { ticks++; fn(...a); }, ms); ativos.add(id); return id; };
	window.clearInterval = (id) => { ativos.delete(id); ci(id); };
	window.__timers = () => ({ ativos: ativos.size, ticks });
};

async function novoContexto() {
	const ctx = await browser.newContext();
	await ctx.addInitScript(INIT);
	const page = await ctx.newPage();
	page.on('pageerror', (e) => erros.push(`pageerror: ${e.message}`));
	page.on('console', (m) => m.type() === 'error' && erros.push(`console: ${m.text()}`));
	let fetches = 0;
	page.on('request', (r) => r.url().endsWith('/produtos.json') && fetches++);
	return { ctx, page, fetches: () => fetches };
}
const H1 = { Painel: 'Painel', Tarefas: 'Tarefas', Loja: 'Loja' };
async function ir(page, nome) {
	await link(page, nome).click();
	await page.waitForURL((u) => u.pathname === { Painel: '/', Tarefas: '/tarefas', Loja: '/loja' }[nome]);
	await page.getByRole('heading', { name: H1[nome], level: 1, exact: true }).waitFor();
}
const link = (page, nome) => page.getByRole('navigation', { name: 'Principal' }).getByRole('link', { name: nome, exact: true });

// ---------- 1. páginas e navegação sem reload ----------
await caso('1. páginas, 404, /loja/:id e navegação client-side', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/');
	await page.getByRole('heading', { name: 'Painel', level: 1 }).waitFor();
	assert.equal(await page.title(), 'Painel · Mercadinho');
	await page.evaluate(() => (window.__marcador = 42));
	assert.equal(await link(page, 'Painel').getAttribute('aria-current'), 'page');

	await ir(page, 'Tarefas');
	await page.getByRole('heading', { name: 'Tarefas', level: 1 }).waitFor();
	assert.equal(await page.title(), 'Tarefas · Mercadinho');
	assert.equal(await link(page, 'Tarefas').getAttribute('aria-current'), 'page');
	assert.equal(await link(page, 'Painel').getAttribute('aria-current'), null);

	await ir(page, 'Loja');
	await page.getByRole('heading', { name: 'Loja', level: 1 }).waitFor();
	assert.equal(await page.title(), 'Loja · Mercadinho');
	await page.getByRole('link', { name: 'Café torrado 500 g' }).click();
	await page.getByRole('heading', { name: 'Café torrado 500 g', level: 1 }).waitFor();
	assert.equal(new URL(page.url()).pathname, '/loja/cafe');
	assert.equal(await page.title(), 'Café torrado 500 g · Mercadinho');
	assert.equal(await link(page, 'Loja').getAttribute('aria-current'), 'true');
	await page.getByRole('link', { name: '← Voltar para a loja' }).click();
	await page.getByRole('heading', { name: 'Loja', level: 1 }).waitFor();
	assert.equal(await page.evaluate(() => window.__marcador), 42, 'página recarregou');

	await page.goto(BASE + '/nao-existe');
	await page.getByRole('heading', { name: 'Página não encontrada' }).waitFor();
	assert.equal(await page.title(), 'Página não encontrada · Mercadinho');
	await page.evaluate(() => (window.__marcador = 7));
	await page.getByRole('link', { name: 'Voltar para o início' }).click();
	await page.getByRole('heading', { name: 'Painel', level: 1 }).waitFor();
	assert.equal(await page.evaluate(() => window.__marcador), 7);

	await page.goto(BASE + '/loja/xyz');
	await page.getByRole('heading', { name: 'Produto não encontrado' }).waitFor();
	assert.equal(await page.title(), 'Produto não encontrado · Mercadinho');
	await ctx.close();
});

// ---------- 2. voltar/avançar e deep link ----------
await caso('2. voltar/avançar e deep links com filtros na URL', async () => {
	const { ctx, page } = await novoContexto();
	const filtro = page.getByRole('group', { name: 'Filtrar tarefas' });
	const titulos = () => page.locator('.todo-list .titulo').allTextContents();
	await page.goto(BASE + '/tarefas?filtro=concluidas');
	await page.getByRole('heading', { name: 'Tarefas', level: 1 }).waitFor();
	assert.deepEqual(await titulos(), ['Pagar contas']);
	assert.equal(await filtro.getByRole('button', { name: 'Concluídas' }).getAttribute('aria-pressed'), 'true');

	await page.evaluate(() => (window.__marcador = 1));
	await filtro.getByRole('button', { name: 'Pendentes' }).click();
	await page.waitForURL(/filtro=pendentes/);
	assert.deepEqual(await titulos(), ['Comprar café', 'Ligar para a farmácia']);
	await filtro.getByRole('button', { name: 'Todas' }).click();
	await page.waitForURL((u) => !u.search.includes('filtro'));
	assert.equal((await titulos()).length, 3);
	await page.goBack();
	await page.waitForURL(/filtro=pendentes/);
	assert.deepEqual(await titulos(), ['Comprar café', 'Ligar para a farmácia']);
	await page.goBack();
	await page.waitForURL(/filtro=concluidas/);
	assert.deepEqual(await titulos(), ['Pagar contas']);
	await page.goForward();
	await page.waitForURL(/filtro=pendentes/);
	assert.equal(await filtro.getByRole('button', { name: 'Pendentes' }).getAttribute('aria-pressed'), 'true');
	assert.equal(await page.evaluate(() => window.__marcador), 1, 'recarregou');

	await page.reload();
	await page.getByRole('heading', { name: 'Tarefas', level: 1 }).waitFor();
	assert.deepEqual(await titulos(), ['Comprar café', 'Ligar para a farmácia']);

	// loja: deep link de categoria, back/forward entre páginas
	await page.goto(BASE + '/loja?categoria=limpeza');
	await page.locator('.produto').first().waitFor();
	assert.equal(await page.locator('.produto').count(), 3);
	await page.getByRole('group', { name: 'Filtrar por categoria' }).getByRole('button', { name: 'Hortifruti' }).click();
	await page.waitForURL(/categoria=hortifruti/);
	assert.equal(await page.locator('.produto h3').first().textContent(), 'Maçã gala 1 kg');
	await page.goBack();
	await page.waitForURL(/categoria=limpeza/);
	assert.equal(await page.locator('.produto').count(), 3);

	// refresh em /loja/cafe
	await page.goto(BASE + '/loja/cafe');
	await page.reload();
	await page.getByRole('heading', { name: 'Café torrado 500 g', level: 1 }).waitFor();
	await page.goto(BASE + '/');
	await page.goBack();
	await page.getByRole('heading', { name: 'Café torrado 500 g', level: 1 }).waitFor();
	await ctx.close();
});

// ---------- 3. Counters ----------
await caso('3. Counters respeitam min/max e são independentes', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/');
	const grupo = (n) => page.getByRole('group', { name: n, exact: true });
	const val = (n) => grupo(n).locator('output').textContent();
	for (let i = 0; i < 3; i++) await grupo('Cliques').getByRole('button', { name: 'Aumentar Cliques' }).click();
	assert.equal(await val('Cliques'), '3');
	assert.equal(await val('Pulos de 5'), '10');
	const p = grupo('Pulos de 5');
	for (let i = 0; i < 8; i++) await p.getByRole('button', { name: 'Aumentar' }).click();
	assert.equal(await val('Pulos de 5'), '50');
	assert.ok(await p.getByRole('button', { name: 'Aumentar' }).isDisabled(), '+ habilitado no max');
	assert.ok(await p.getByRole('button', { name: 'Diminuir' }).isEnabled());
	for (let i = 0; i < 10; i++) await p.getByRole('button', { name: 'Diminuir' }).click();
	assert.equal(await val('Pulos de 5'), '0');
	assert.ok(await p.getByRole('button', { name: 'Diminuir' }).isDisabled(), '− habilitado no min');
	await page.getByRole('tab', { name: 'Contador' }).click();
	assert.equal(await val('Na aba'), '3');
	await grupo('Na aba').getByRole('button', { name: 'Aumentar' }).click();
	assert.equal(await val('Na aba'), '6');
	assert.equal(await val('Cliques'), '3');
	await page.getByRole('tab', { name: 'Sobre' }).click();
	await page.getByRole('tab', { name: 'Contador' }).click();
	assert.equal(await val('Na aba'), '6', 'contador da aba perdeu estado');

	await ir(page, 'Loja');
	const card = page.locator('.produto', { hasText: 'Alface crespa' });
	await card.waitFor();
	const qtd = card.getByRole('group', { name: 'Qtd' });
	assert.equal(await qtd.locator('output').textContent(), '1');
	assert.ok(await qtd.getByRole('button', { name: 'Diminuir Qtd' }).isDisabled());
	await qtd.getByRole('button', { name: 'Aumentar Qtd' }).click();
	await qtd.getByRole('button', { name: 'Aumentar Qtd' }).click();
	assert.equal(await qtd.locator('output').textContent(), '3');
	assert.ok(await qtd.getByRole('button', { name: 'Aumentar Qtd' }).isDisabled(), 'passou do estoque');
	assert.equal(await page.locator('.produto', { hasText: 'Banana' }).getByRole('group', { name: 'Qtd' }).locator('output').textContent(), '1');
	const det = page.locator('.produto', { hasText: 'Detergente' });
	assert.ok(await det.getByText('Esgotado').isVisible());
	assert.ok(await det.getByRole('button', { name: 'Adicionar' }).isDisabled());
	await ctx.close();
});

// ---------- 4. Carrinho -> badge -> painel ----------
await caso('4. carrinho atualiza CartBadge na hora e StatCards do Painel', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/');
	const badge = page.locator('.cart-badge .count');
	assert.match(await badge.textContent(), /^0/);
	assert.equal(await page.locator('.stat', { hasText: 'Itens no carrinho' }).getAttribute('data-tone'), 'alerta');
	await ir(page, 'Loja');
	const cafe = page.locator('.produto', { hasText: 'Café torrado' });
	await cafe.getByRole('button', { name: 'Aumentar Qtd' }).click();
	await cafe.getByRole('button', { name: 'Adicionar' }).click();
	assert.match(await badge.textContent(), /^2/);
	const alface = page.locator('.produto', { hasText: 'Alface' });
	for (let i = 0; i < 2; i++) await alface.getByRole('button', { name: 'Aumentar Qtd' }).click();
	await alface.getByRole('button', { name: 'Adicionar' }).click();
	await alface.getByRole('button', { name: 'Adicionar' }).isDisabled();
	assert.match(await badge.textContent(), /^5/);
	// CartSummary: + no limite do estoque, − e remover
	const cart = page.getByRole('complementary');
	const lin = cart.locator('li', { hasText: 'Alface' });
	assert.ok(await lin.getByRole('button', { name: 'Aumentar' }).isDisabled(), 'carrinho passou do estoque');
	await lin.getByRole('button', { name: 'Diminuir' }).click();
	assert.match(await badge.textContent(), /^4/);
	assert.match(await cart.locator('.total').textContent(), /R\$\s?74,80/);
	await ir(page, 'Painel');
	const stat = (t) => page.locator('.stat', { hasText: t }).locator('.value').textContent();
	assert.equal(await stat('Itens no carrinho'), '4');
	assert.match(await stat('Total do carrinho'), /R\$\s?74,80/);
	assert.equal(await stat('Tarefas pendentes'), '2');
	assert.equal(await page.locator('.stat', { hasText: 'Itens no carrinho' }).getAttribute('data-tone'), 'neutro');
	await ir(page, 'Loja');
	await cart.locator('li', { hasText: 'Alface' }).getByRole('button', { name: /Remover/ }).click();
	assert.match(await badge.textContent(), /^2/);
	await cart.getByRole('button', { name: 'Esvaziar' }).click();
	assert.match(await badge.textContent(), /^0/);
	assert.ok(await cart.getByText('Carrinho vazio.').isVisible());
	await ctx.close();
});

// ---------- 5. Tarefas ----------
await caso('5. tarefas: adicionar, validar, concluir, editar, remover, buscar, filtrar, limpar', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/tarefas');
	const campo = page.getByLabel('Nova tarefa');
	const erro = page.locator('.todo-form .field-error');
	const titulos = () => page.locator('.todo-list .titulo').allTextContents();
	await campo.press('Enter');
	assert.match(await erro.textContent(), /Informe/);
	assert.equal(await campo.getAttribute('aria-invalid'), 'true');
	await campo.fill('ab'); await campo.press('Enter');
	assert.match(await erro.textContent(), /3 caracteres/);
	await campo.fill('x'.repeat(81)); await campo.press('Enter');
	assert.match(await erro.textContent(), /80/);
	await campo.fill('  COMPRAR CAFÉ '); await campo.press('Enter');
	assert.match(await erro.textContent(), /Já existe/);
	await campo.fill('Lavar a louça');
	assert.equal(await erro.count(), 0, 'erro não sumiu ao digitar');
	await campo.press('Enter');
	assert.deepEqual((await titulos()).at(-1), 'Lavar a louça');
	assert.equal(await campo.inputValue(), '');
	assert.ok(await campo.evaluate((el) => el === document.activeElement), 'foco não voltou');
	await page.getByRole('button', { name: 'Adicionar' }).click();
	assert.equal(await erro.count(), 1, 'botão Adicionar não valida');

	// concluir
	await page.getByRole('checkbox', { name: 'Concluir “Lavar a louça”' }).check();
	assert.equal(await page.locator('.stat .value').textContent(), '2 de 4');
	// editar: Enter salva
	await page.locator('.titulo', { hasText: 'Lavar a louça' }).dblclick();
	const edit = page.getByRole('textbox', { name: /Editar tarefa/ });
	assert.ok(await edit.evaluate((el) => el === document.activeElement));
	await edit.fill('Lavar a louça hoje'); await edit.press('Enter');
	assert.ok((await titulos()).includes('Lavar a louça hoje'));
	// editar: Esc cancela
	await page.locator('.titulo', { hasText: 'Comprar café' }).dblclick();
	await edit.fill('Outro nome'); await edit.press('Escape');
	assert.ok((await titulos()).includes('Comprar café'));
	assert.ok(!(await titulos()).includes('Outro nome'));
	// editar: duplicata é rejeitada
	await page.locator('.titulo', { hasText: 'Comprar café' }).dblclick();
	await edit.fill('pagar contas'); await edit.press('Enter');
	assert.match(await page.locator('.todo-list .field-error').textContent(), /Já existe/);
	await edit.press('Escape');
	// buscar
	await page.getByRole('searchbox', { name: 'Buscar' }).fill('LAVAR');
	assert.deepEqual(await titulos(), ['Lavar a louça hoje']);
	await page.getByRole('button', { name: 'Pendentes' }).click();
	await page.waitForURL(/filtro=pendentes/);
	assert.ok(await page.getByText('Nenhuma tarefa').isVisible());
	await page.getByRole('searchbox', { name: 'Buscar' }).fill('');
	assert.deepEqual(await titulos(), ['Comprar café', 'Ligar para a farmácia']);
	await page.getByRole('button', { name: 'Todas' }).click();
	// remover
	await page.getByRole('button', { name: 'Remover “Ligar para a farmácia”' }).click();
	assert.ok(!(await titulos()).includes('Ligar para a farmácia'));
	// limpar concluídas: cancelar não faz nada
	const limpar = page.getByRole('button', { name: 'Limpar concluídas' });
	await limpar.click();
	const dlg = page.getByRole('dialog', { name: 'Limpar concluídas?' });
	await dlg.waitFor();
	await dlg.getByRole('button', { name: 'Cancelar' }).click();
	assert.ok(!(await dlg.isVisible()));
	assert.equal((await titulos()).length, 3);
	assert.ok(await limpar.evaluate((el) => el === document.activeElement), 'foco não voltou ao botão');
	await limpar.click();
	await dlg.getByRole('button', { name: 'Confirmar' }).click();
	assert.deepEqual(await titulos(), ['Comprar café']);
	assert.ok(await limpar.isDisabled());
	await ctx.close();
});

// ---------- 6. Refresh mantém estado ----------
await caso('6. refresh mantém tarefas, carrinho e tema', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/');
	const temaInicial = await page.evaluate(() => document.documentElement.dataset.theme);
	await page.getByRole('button', { name: 'Tema escuro' }).click();
	const temaNovo = temaInicial === 'dark' ? 'light' : 'dark';
	assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), temaNovo);
	await ir(page, 'Tarefas');
	await page.getByLabel('Nova tarefa').fill('Persistir isto');
	await page.getByLabel('Nova tarefa').press('Enter');
	await page.getByRole('checkbox', { name: 'Concluir “Comprar café”' }).check();
	await ir(page, 'Loja');
	await page.locator('.produto', { hasText: 'Banana' }).getByRole('button', { name: 'Adicionar' }).click();
	await page.reload();
	await page.locator('.produto').first().waitFor();
	assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), temaNovo);
	assert.match(await page.locator('.cart-badge .count').textContent(), /^1/);
	assert.ok(await page.getByRole('complementary').locator('.nome', { hasText: 'Banana prata 1 kg' }).isVisible());
	await page.goto(BASE + '/tarefas');
	await page.locator('.todo-list').waitFor();
	assert.ok((await page.locator('.todo-list .titulo').allTextContents()).includes('Persistir isto'));
	assert.ok(await page.getByRole('checkbox', { name: 'Concluir “Comprar café”' }).isChecked());
	await page.goto(BASE + '/');
	assert.equal(await page.getByRole('button', { name: 'Tema escuro' }).getAttribute('aria-pressed'), String(temaNovo === 'dark'));
	await ctx.close();
});

// ---------- 7. Clock para ao sair ----------
await caso('7. Clock: timer para ao sair do Painel (sem vazamento)', async () => {
	const { ctx, page } = await novoContexto();
	// linha de base fora do Painel (o dev server do Vite tem timers próprios)
	await page.goto(BASE + '/tarefas');
	await page.getByRole('heading', { name: 'Tarefas', level: 1 }).waitFor();
	await page.waitForTimeout(300);
	const base = (await page.evaluate(() => window.__timers())).ativos;
	await ir(page, 'Painel');
	const hora = page.locator('.clock time');
	await hora.waitFor();
	const h1 = await hora.textContent();
	assert.match(h1, /^\d{2}:\d{2}:\d{2}$/);
	await page.waitForTimeout(1600);
	assert.notEqual(await hora.textContent(), h1, 'relógio não atualiza');
	const noPainel = await page.evaluate(() => window.__timers());
	assert.equal(noPainel.ativos, base + 1, `intervalos ativos no painel: ${noPainel.ativos} (base ${base})`);
	await ir(page, 'Tarefas');
	await page.getByRole('heading', { name: 'Tarefas', level: 1 }).waitFor();
	const saiu = await page.evaluate(() => window.__timers());
	await page.waitForTimeout(2500);
	const depois = await page.evaluate(() => window.__timers());
	assert.equal(saiu.ativos, base, 'intervalo não foi limpo');
	assert.equal(depois.ticks - saiu.ticks, 0, `ticks continuaram: ${saiu.ticks} -> ${depois.ticks}`);
	// ida e volta 5x não acumula timers
	for (let i = 0; i < 5; i++) { await ir(page, 'Painel'); await ir(page, 'Loja'); }
	await ir(page, 'Painel');
	assert.equal((await page.evaluate(() => window.__timers())).ativos, base + 1);
	results.at(-1); console.log(`   [7] base fora do painel: ${base}; painel: ${noPainel.ativos} ativo(s), ${noPainel.ticks} ticks; após sair: ${saiu.ativos} ativo(s), ticks ${saiu.ticks} -> ${depois.ticks} em 2,5 s`);
	await ctx.close();
});

// ---------- 8. Esqueleto, erro e cache ----------
await caso('8. esqueleto na carga, erro com ?falhar=1, cache ao voltar', async () => {
	const { ctx, page, fetches } = await novoContexto();
	await page.goto(BASE + '/loja');
	await page.locator('.skeleton').first().waitFor({ timeout: 300 });
	assert.equal(await page.locator('.grid-produtos').getAttribute('aria-busy'), 'true');
	const t0 = Date.now();
	await page.locator('.produto').first().waitFor();
	assert.equal(await page.locator('.skeleton').count(), 0);
	assert.equal(await page.locator('.produto').count(), 9);
	assert.equal(fetches(), 1);
	await page.getByRole('searchbox', { name: 'Buscar' }).fill('zzz');
	assert.ok(await page.getByText('Nenhum produto').isVisible());
	await ir(page, 'Painel');
	await ir(page, 'Loja');
	assert.equal(await page.locator('.skeleton').count(), 0, 'esqueleto ao voltar (recarregou)');
	assert.equal(await page.locator('.produto').count(), 9);
	await page.getByRole('link', { name: 'Arroz integral 1 kg' }).click();
	await page.getByRole('heading', { name: 'Arroz integral 1 kg' }).waitFor();
	await page.goBack();
	await page.locator('.produto').first().waitFor();
	assert.equal(fetches(), 1, `fetches: ${fetches()}`);

	const f2 = await novoContexto();
	await f2.page.goto(BASE + '/loja?falhar=1');
	await f2.page.locator('.skeleton').first().waitFor({ timeout: 300 });
	await f2.page.getByRole('alert').waitFor();
	assert.match(await f2.page.getByRole('alert').textContent(), /Não foi possível carregar/);
	await f2.page.getByRole('button', { name: 'Tentar de novo' }).click();
	await f2.page.locator('.skeleton').first().waitFor({ timeout: 300 });
	await f2.page.getByRole('alert').waitFor();
	// tira ?falhar da URL e tenta de novo: carrega
	await f2.page.getByRole('button', { name: 'Hortifruti' }).click();
	await f2.page.goto(BASE + '/loja');
	await f2.page.locator('.produto').first().waitFor();
	console.log(`   [8] fetches de produtos.json numa sessão com 3 visitas à loja: ${fetches()}`);
	await f2.ctx.close();
	await ctx.close();
});

// ---------- 9. Modal e Tabs pelo teclado ----------
await caso('9. modal fecha com Esc/Fechar/fundo e devolve foco; tabs com teclado', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/');
	const abrir = page.getByRole('button', { name: 'Sobre o app' });
	const dlg = page.getByRole('dialog', { name: 'Sobre o Mercadinho' });
	const focado = (loc) => loc.evaluate((el) => el === document.activeElement);
	await abrir.focus(); await page.keyboard.press('Enter');
	await dlg.waitFor();
	await page.keyboard.press('Escape');
	await dlg.waitFor({ state: 'hidden' });
	assert.ok(await focado(abrir), 'foco não voltou após Esc');
	await abrir.click(); await dlg.waitFor();
	await dlg.getByRole('button', { name: 'Fechar' }).click();
	await dlg.waitFor({ state: 'hidden' });
	assert.ok(await focado(abrir), 'foco não voltou após Fechar');
	await abrir.click(); await dlg.waitFor();
	await page.mouse.click(5, 5); // fundo
	await dlg.waitFor({ state: 'hidden' });
	assert.ok(await focado(abrir), 'foco não voltou após clique no fundo');
	await abrir.click(); await dlg.waitFor();
	await dlg.getByRole('heading').click(); // clique dentro não fecha
	assert.ok(await dlg.isVisible(), 'fechou com clique dentro');
	await page.keyboard.press('Escape');
	await dlg.waitFor({ state: 'hidden' });
	await page.waitForFunction(() => document.activeElement?.textContent === 'Sobre o app');

	const tab = (n) => page.getByRole('tab', { name: n });
	await tab('Sobre').focus();
	await page.keyboard.press('ArrowRight');
	assert.equal(await tab('Atalhos').getAttribute('aria-selected'), 'true');
	assert.ok(await focado(tab('Atalhos')));
	assert.ok(await page.getByRole('tabpanel', { name: 'Atalhos' }).isVisible());
	assert.ok(!(await page.getByText('Mercadinho é uma SPA').isVisible()));
	await page.keyboard.press('ArrowRight');
	assert.equal(await tab('Contador').getAttribute('aria-selected'), 'true');
	await page.keyboard.press('ArrowRight'); // circular
	assert.equal(await tab('Sobre').getAttribute('aria-selected'), 'true');
	await page.keyboard.press('ArrowLeft');
	assert.equal(await tab('Contador').getAttribute('aria-selected'), 'true');
	assert.equal(await tab('Sobre').getAttribute('tabindex'), '-1');
	await page.keyboard.press('Home');
	assert.ok(await focado(tab('Sobre')));
	await page.keyboard.press('Tab'); // vai para o painel
	assert.ok(await focado(page.getByRole('tabpanel', { name: 'Sobre' })));
	await ctx.close();
});

// ---------- extra: CSS em camadas e tema ----------
await caso('extra: camadas CSS e tokens do tema escuro aplicados', async () => {
	const { ctx, page } = await novoContexto();
	await page.goto(BASE + '/');
	await page.getByRole('heading', { name: 'Painel', level: 1 }).waitFor();
	const bgClaro = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
	await page.evaluate(() => (document.documentElement.dataset.theme = 'dark'));
	const bgEscuro = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
	assert.notEqual(bgClaro, bgEscuro);
	const ordem = await page.evaluate(() => [...document.styleSheets].flatMap((s) => [...s.cssRules]).filter((r) => r.constructor.name === 'CSSLayerStatementRule').map((r) => r.nameList.join(',')));
	assert.equal(ordem[0], 'reset,tokens,base,layout,components,utilities');
	await ctx.close();
});

await browser.close();
for (const r of results) console.log(r.join('  '));
console.log(erros.length ? `Erros no console:\n${[...new Set(erros)].join('\n')}` : 'Sem erros no console.');
process.exit(results.some((r) => r[0] === '❌') ? 1 : 0);
