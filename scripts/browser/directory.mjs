// Check the generated directory through HTTP, including the no-script fallback.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve, join, extname, sep } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(process.argv[2] || 'target/pages');
const captures = resolve(process.argv[3] || 'target/directory-captures');
const entries = JSON.parse(await readFile('site/directory.json', 'utf8'));
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  if (url.pathname === '/favicon.ico') return response.writeHead(204).end();
  let path = resolve(root, '.' + decodeURIComponent(url.pathname));
  if (!path.startsWith(root + sep)) return response.writeHead(404).end();
  if (url.pathname.endsWith('/')) path = join(path, 'index.html');
  try {
    const body = await readFile(path);
    const mime = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript' };
    response.writeHead(200, { 'Content-Type': mime[extname(path)] || 'application/octet-stream' }).end(body);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const url = 'http://127.0.0.1:' + server.address().port + '/directory/';
const browser = await chromium.launch({ headless: true });
await mkdir(captures, { recursive: true });
try {
  for (const viewport of [{ width: 1280, height: 900 }, { width: 390, height: 844 }]) {
    const context = await browser.newContext({ viewport });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => {
      if (response.status() >= 400) errors.push(response.url() + ' ' + response.status());
    });
    await page.goto(url);
    const shown = page.locator('.directory-entry:not([hidden])');
    assert.equal(await shown.count(), entries.length);
    await page.locator('[data-category="demos"]').click();
    assert.equal(await shown.count(), entries.filter(entry => entry.category === 'demos').length);
    await page.locator('#directory-search').fill('Tween Lab');
    assert.equal(await shown.count(), 1);
    assert.match(await shown.first().textContent(), /Tween Lab/);
    assert.equal(new URL(page.url()).searchParams.get('type'), 'demos');
    await page.reload();
    assert.equal(await shown.count(), 1, 'shared filter URL');
    await page.locator('#directory-search').fill('no-such-entry');
    assert.equal(await shown.count(), 0);
    assert.equal(await page.locator('#directory-empty').isVisible(), true);
    await page.locator('#directory-reset').click();
    assert.equal(await shown.count(), entries.length);
    await page.locator('[data-category="docs"]').click();
    await page.locator('#directory-search').fill('Weave');
    assert.equal(await shown.count(), 2);
    await page.locator('#directory-search').fill('');
    await page.locator('[data-category="all"]').click();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.screenshot({ path: join(captures, 'directory-' + viewport.width + '.png') });
    assert.deepEqual(errors, []);
    await context.close();
  }
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto(url);
  assert.equal(await page.locator('.directory-entry').count(), entries.length);
  assert.equal(await page.locator('.directory-controls').isVisible(), false);
  assert.equal(await page.locator('noscript').isVisible(), true);
  await context.close();
  console.log('directory search, filters, shared URLs, mobile layout and no-script fallback passed');
} finally {
  await browser.close();
  await new Promise(done => server.close(done));
}
