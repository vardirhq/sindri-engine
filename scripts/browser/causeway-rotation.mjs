// Exercise the exported battlefield through keyboard, touch buttons and a sea swipe.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(process.argv[2]);
const base = process.env.SINDRI_BASE_PATH || '/';
const output = resolve(process.argv[3] || 'target/causeway-review');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
  if (path === '/favicon.ico') { response.writeHead(204).end(); return; }
  try {
    if (!path.startsWith(base)) { response.writeHead(404).end(); return; }
    const file = join(root, path.slice(base.length) || 'index.html');
    response.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream' }).end(await readFile(file));
  } catch { response.writeHead(404).end(); }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
await mkdir(output, { recursive: true });
let browser;
try {
  browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || undefined,
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--use-vulkan=swiftshader', '--enable-gpu', '--ignore-gpu-blocklist', '--no-sandbox'],
  });
  for (const [name, viewport] of [['desktop', { width: 1280, height: 720 }], ['phone', { width: 390, height: 844 }]]) {
    const context = await browser.newContext({ viewport, hasTouch: name === 'phone' });
    const page = await context.newPage();
    const errors = [];
    const turns = [];
    let battlefieldReady = false;
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      if (message.text().includes('Causeway battlefield ready')) battlefieldReady = true;
      if (message.text().includes('Causeway view settled:')) turns.push(message.text());
      if (message.type() === 'error' || /error while parsing wgsl|is invalid|InvalidTransform/.test(message.text())) errors.push(message.text());
    });
    await page.goto(`http://127.0.0.1:${server.address().port}${base}`);
    await page.waitForFunction(() => !document.querySelector('#sindri-loading'), null, { timeout: 60000 });
    await page.waitForTimeout(5000);
    await page.keyboard.press('Enter');
    for (let attempt = 0; attempt < 300 && !battlefieldReady; attempt++) await page.waitForTimeout(100);
    assert(battlefieldReady, `${name} title handed the camera to the battlefield`);
    const waitForTurns = async (count) => {
      for (let attempt = 0; attempt < 100 && turns.length < count; attempt++) await page.waitForTimeout(100);
      assert.equal(turns.length, count, `${name} completed ${count} quarter turns`);
    };
    const before = await page.locator('canvas').screenshot();
    if (name === 'desktop') {
      await page.keyboard.press('l');
    } else {
      await page.touchscreen.tap(359, 102);
    }
    await page.waitForTimeout(1000);
    await waitForTurns(1);
    const after = await page.locator('canvas').screenshot();
    assert(!before.equals(after), `${name} view changes after rotation input`);
    if (name === 'phone') {
      const cdp = await context.newCDPSession(page);
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x: 80, y: 180 }] });
      await page.waitForTimeout(100);
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: 180, y: 180 }] });
      await page.waitForTimeout(100);
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
      await page.waitForTimeout(1000);
      await waitForTurns(2);
    }
    await page.screenshot({ path: join(output, `browser-${name}.png`) });
    assert.deepEqual(errors, [], `${name} has no runtime or GPU errors`);
    console.log(`Causeway ${name}: rotation input and rendered battlefield passed`);
    await context.close();
  }
} finally {
  if (browser) await browser.close();
  await new Promise((done) => server.close(done));
}
