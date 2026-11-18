import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || '/tmp/pragma-astra-browser/node_modules/playwright/index.mjs'));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', args: ['--no-sandbox'] });
try {
  const page = await browser.newPage();
  await page.goto('http://127.0.0.1:5172/tarefas?filtro=concluidas');
  assert.equal(await page.locator('.todo-list > li').count(), 1);
  await page.reload();
  assert.equal(await page.locator('.todo-list > li').count(), 1);
  await page.goto('http://127.0.0.1:5172/loja/cafe');
  await page.locator('.product-card').waitFor();
  assert.match(await page.title(), /Café/);
  await page.reload();
  await page.locator('.product-card').waitFor();
  await page.getByRole('link', { name: '← Voltar para a loja' }).click();
  await page.getByRole('button', { name: 'Limpeza', exact: true }).click();
  assert.equal(await page.locator('.product-card').count(), 3);
  await page.goBack();
  await page.waitForURL('**/loja');
  assert.equal(await page.locator('.product-card').count(), 9);
  await page.goForward();
  await page.waitForURL('**/loja?categoria=limpeza');
  assert.equal(await page.locator('.product-card').count(), 3);
  await writeFile('evidence/dev-deep-links.json', JSON.stringify({ passed: true, mode: 'dev', checked: ['deep link e refresh /tarefas?filtro=concluidas', 'deep link e refresh /loja/cafe', 'voltar/avançar categoria da loja'] }, null, 2));
  console.log('PASS deep links, refresh e histórico no dev');
} finally { await browser.close(); }
