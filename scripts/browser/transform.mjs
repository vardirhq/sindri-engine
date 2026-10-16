// Transform Lab through the exported shared WebGPU player.
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
    channel: 'chromium',
    executablePath: process.env.CHROME_PATH || undefined,
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--use-vulkan=swiftshader', '--enable-gpu', '--ignore-gpu-blocklist', '--no-sandbox'],
  });
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const logs = [], errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    logs.push(message.text());
    if (message.type() === 'error' || /error while parsing wgsl|is invalid|must only be called/i.test(message.text())) errors.push(message.text());
  });
  const waitFor = async (words) => {
    for (let i = 0; i < 200 && !logs.some((line) => line.includes(words)); i++) await page.waitForTimeout(100);
    assert(logs.some((line) => line.includes(words)), `missing ${words}: ${logs.slice(-10).join(' | ')}`);
  };
  await page.goto(origin + base);
  await waitFor('Quaternion saved');
  const before = await page.locator('canvas').screenshot();
  await waitFor('Quaternion copied and restored');
  await waitFor('Arrived');
  assert(!logs.some((line) => line.includes('Quaternion mismatch')), 'copied and restored values agree');
  assert(!before.equals(await page.locator('canvas').screenshot()), 'the walker and orbit changed the rendered scene');
  const pixels = await page.screenshot(shot ? { path: shot } : {});
  assert(imageStatistics(pixels).colors >= 8, 'the lab renders several scene colors');
  for (const asset of ['transform.scene', 'transform.decay', 'walker.png', 'goal.png', 'floor.png']) {
    assert([...fetched].some((path) => path.endsWith('/' + asset)), `fetched ${asset}`);
  }
  assert.deepEqual(errors, [], 'no runtime or GPU errors');
  console.log('Transform Lab: fetched assets, quaternion save/copy/restore, walker goal and rendered motion passed');
} finally {
  if (browser) await browser.close();
  await new Promise((done) => server.close(done));
}
