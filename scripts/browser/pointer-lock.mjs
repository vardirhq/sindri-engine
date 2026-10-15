// Real cursor capture in the exported Orbit Camera Lab, including browser denial.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { chromium } from 'playwright';
import { imageStatistics } from './png.mjs';

const root = resolve(process.argv[2]);
const shot = process.argv[3];
const base = process.env.SINDRI_BASE_PATH || '/';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const fetched = new Set();
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
  if (path === '/denied.html') {
    response.writeHead(200, { 'content-type': 'text/html' }).end(
      `<iframe id="denied" sandbox="allow-scripts allow-same-origin" src="${base}" width="960" height="540"></iframe>`,
    );
    return;
  }
  if (path === '/favicon.ico') { response.writeHead(204).end(); return; }
  if (!path.startsWith(base)) { response.writeHead(404).end(); return; }
  const file = join(root, path.slice(base.length) || 'index.html');
  try {
    const body = await readFile(file);
    fetched.add(path);
    response.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream' }).end(body);
  } catch { response.writeHead(404).end(); }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
try {
  browser = await chromium.launch({
    // Full Chromium's headless mode supports captured relative mouse input.
    // The separate headless shell reports cursor warps for CDP mouse moves.
    channel: 'chromium',
    executablePath: process.env.CHROME_PATH || undefined,
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--use-vulkan=swiftshader', '--enable-gpu', '--ignore-gpu-blocklist', '--no-sandbox'],
  });
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const logs = [], errors = [], motion = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    const text = message.text();
    logs.push(text);
    if (message.type() === 'error') errors.push(text);
    const match = /Pointer motion ([-\d.]+) ([-\d.]+)/.exec(text);
    if (match) motion.push([Number(match[1]), Number(match[2])]);
  });
  await page.goto(origin + base);
  await page.waitForFunction(() => document.querySelector('canvas') && !document.querySelector('#sindri-loading'));
  await page.mouse.click(200, 200);
  await page.waitForFunction(() => document.pointerLockElement === document.querySelector('canvas'));
  await page.waitForTimeout(150);
  await page.mouse.move(1600, 240, { steps: 4 });
  await page.mouse.move(1620, 260);
  await page.waitForTimeout(500);
  const sum = motion.reduce(([x, y], [dx, dy]) => [x + dx, y + dy], [0, 0]);
  assert.deepEqual(sum, [1420, 60], `unbounded motion reached Decay once: ${JSON.stringify(motion)}`);
  await page.keyboard.press('u');
  await page.waitForFunction(() => !document.pointerLockElement);
  // DOM release precedes the next shared fixed step and console delivery.
  for (let i = 0; i < 100 && !logs.some((text) => text.includes('Pointer released')); i++) {
    await page.waitForTimeout(100);
  }
  assert(logs.some((text) => text.includes('Pointer released')),
    `Decay saw explicit release: ${logs.slice(-10).join(' | ')}`);
  await page.mouse.click(200, 200);
  await page.waitForFunction(() => document.pointerLockElement);
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.pointerLockElement);
  await page.waitForTimeout(300);
  assert.equal(await page.evaluate(() => document.pointerLockElement), null, 'Escape does not automatically recapture');
  for (let i = 0; i < 120 && !logs.some((text) => text.includes('Arrived')); i++) await page.waitForTimeout(100);
  assert(logs.some((text) => text.includes('Arrived')), 'the project reaches its goal');
  // Resize through gameplay input after the automatic barrier has disappeared.
  // The script waits for synchronized center/edge rays, so a missing collider
  // cannot count as a successful shrink. Every edit needs a new query result.
  await page.keyboard.press('3'); // Look back at the wall from the walker's far side.
  await page.waitForTimeout(750); // Let the shared orbit smoothing settle.
  for (const [key, state] of [['1', 'narrow'], ['2', 'wide'], ['1', 'narrow']]) {
    const message = `Barrier scale query ${state}`;
    const before = logs.filter(text => text.includes(message)).length;
    await page.keyboard.press(key);
    for (let i = 0; i < 100 && logs.filter(text => text.includes(message)).length <= before; i++) {
      await page.waitForTimeout(100);
    }
    assert(logs.filter(text => text.includes(message)).length > before,
      `scale edit ${state} reaches synchronized center/edge collision queries`);
    await page.waitForTimeout(150);
    if (shot) await page.screenshot({ path: shot.replace(/\.png$/, '') + `-scale-${state}.png` });
  }
  await page.keyboard.press('r');
  for (let i = 0; i < 50 && !logs.some(text => text.includes('Barrier automatic')); i++) {
    await page.waitForTimeout(100);
  }
  assert(logs.some(text => text.includes('Barrier automatic')), 'R restores the automatic barrier');
  await page.waitForTimeout(2500);
  assert.deepEqual(errors, [], 'no runtime or GPU errors');
  const pixels = await page.screenshot(shot ? { path: shot } : {});
  // This tiny flat-color fixture has fewer colors than the general game gate.
  assert(imageStatistics(pixels).colors >= 8, 'the lab draws several scene colors');
  assert([...fetched].some((path) => path.includes('/assets/')), 'project assets were fetched');

  const denied = await browser.newPage({ viewport: { width: 1000, height: 600 } });
  const deniedLogs = [], deniedErrors = [];
  denied.on('pageerror', (error) => deniedErrors.push(error.message));
  denied.on('console', (message) => {
    deniedLogs.push(message.text());
    if (message.type() === 'error' && !/pointer.?lock|sandbox/i.test(message.text())) deniedErrors.push(message.text());
  });
  await denied.goto(origin + '/denied.html');
  const frame = denied.frames().find((frame) => frame !== denied.mainFrame());
  assert(frame, 'sandboxed game frame exists');
  await frame.waitForFunction(() => document.querySelector('canvas') && !document.querySelector('#sindri-loading'));
  await frame.locator('canvas').click({ position: { x: 200, y: 200 } });
  await denied.waitForTimeout(500);
  assert.equal(await frame.evaluate(() => document.pointerLockElement), null, 'sandbox denies capture');
  assert(!deniedLogs.some((text) => text.includes('Pointer captured')), 'Decay never reports false success');
  for (let i = 0; i < 120 && !deniedLogs.some((text) => text.includes('Arrived')); i++) await denied.waitForTimeout(100);
  assert(deniedLogs.some((text) => text.includes('Arrived')), 'denial leaves gameplay running');
  assert.deepEqual(deniedErrors, [], 'denial produces no unrelated runtime errors');
  console.log(`Pointer lock: unbounded ${sum} motion, explicit release, Escape, sandbox denial, synchronized scale edits and game goal passed`);
} finally {
  if (browser) await browser.close();
  await new Promise((done) => server.close(done));
}
