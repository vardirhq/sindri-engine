// Exercise the exported battlefield through keyboard and touch sea swipes.
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
    let titleReady = false;
    let battlefieldReady = false;
    let swipes = 0;
    page.on('pageerror', (error) => {
      errors.push(error.message);
      console.error(`${name}: ${error.message}`);
    });
    page.on('console', (message) => {
      if (message.text().includes('Causeway title ready')) titleReady = true;
      if (message.text().includes('Causeway sea swipe began')) swipes++;
      if (message.text().includes('Causeway battlefield ready')) battlefieldReady = true;
      if (message.text().includes('Causeway view settled:')) turns.push(message.text());
      if (message.type() === 'error' || /error while parsing wgsl|is invalid|InvalidTransform/.test(message.text())) errors.push(message.text());
    });
    await page.goto(`http://127.0.0.1:${server.address().port}${base}`);
    await page.waitForFunction(() => !document.querySelector('#sindri-loading'), null, { timeout: 60000 });
    // CI renders on a software GPU at around a frame a second, and the game
    // steps a fixed 1/60 s per frame: the title's 1.7 s handover is a hundred
    // frames. Before the island moods it already took 78 s of a 90 s wait.
    const waitFor = async (ready, label) => {
      for (let attempt = 0; attempt < 2400 && !ready(); attempt++) await page.waitForTimeout(100);
      if (!ready()) {
        await page.screenshot({ path: join(output, `failed-${name}.png`), timeout: 90000 });
        assert.fail(`${name}: ${label}; runtime errors: ${JSON.stringify(errors)}`);
      }
    };
    // The loading overlay can disappear before the first scripted title step.
    // Wait for that step, and keep inputs down until gameplay acknowledges them.
    await waitFor(() => titleReady, 'title ready');
    console.log(`${name}: title initialized`);
    await page.locator('canvas').focus();
    await page.keyboard.down('Enter');
    await waitFor(() => battlefieldReady, 'title handed the camera to the battlefield');
    await page.keyboard.up('Enter');
    console.log(`${name}: battlefield ready`);
    const waitForTurns = async (count) => {
      await waitFor(() => turns.length >= count, `completed ${count} quarter turns`);
      assert.equal(turns.length, count, `${name} completes exactly one turn per input`);
    };
    const swipe = async (from, to, count) => {
      const cdp = await context.newCDPSession(page);
      const previous = swipes;
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [from] });
      await waitFor(() => swipes > previous, 'sea owns the touch gesture');
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [to] });
      await waitForTurns(count);
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
      await cdp.detach();
    };
    const captureCanvas = async () => page.screenshot({
      clip: await page.locator('canvas').boundingBox(), timeout: 90000,
    });
    const before = await captureCanvas();
    if (name === 'desktop') {
      await page.keyboard.down('l');
      await waitForTurns(1);
      await page.keyboard.up('l');
    } else {
      // Leave a mouse position behind, as on a touchscreen laptop. Touch must
      // still own the pointer. Mobile rotation is exercised through sea swipes.
      await page.mouse.move(359, 102);
      await swipe({ x: 80, y: 180 }, { x: 180, y: 180 }, 1);
    }
    const after = await captureCanvas();
    assert(!before.equals(after), `${name} view changes after rotation input`);
    await page.screenshot({ path: join(output, `browser-${name}.png`), timeout: 90000 });
    assert.deepEqual(errors, [], `${name} has no runtime or GPU errors`);
    console.log(`Causeway ${name}: rotation input and rendered battlefield passed`);
    await context.close();
  }
} finally {
  if (browser) await browser.close();
  await new Promise((done) => server.close(done));
}
