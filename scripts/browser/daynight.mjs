// Run the exported project through its own clock, or capture a frozen control.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { chromium } from 'playwright';
import { imageStatistics } from './png.mjs';

const root = resolve(process.argv[2]), shot = process.argv[3];
const base = '/examples/daynight/';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const fetched = new Set();
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
  if (path === '/favicon.ico') { response.writeHead(204).end(); return; }
  if (!path.startsWith(base)) { response.writeHead(404).end(); return; }
  try {
    const file = join(root, path.slice(base.length) || 'index.html');
    const body = await readFile(file);
    fetched.add(path);
    response.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream' }).end(body);
  } catch { response.writeHead(404).end(); }
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
let browser;
try {
  browser = await chromium.launch({ channel: 'chromium', executablePath: process.env.CHROME_PATH || undefined,
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--use-vulkan=swiftshader', '--enable-gpu', '--ignore-gpu-blocklist', '--no-sandbox'] });
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const logs = [], errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    logs.push(message.text());
    if (message.type() === 'error' || /error while parsing wgsl|is invalid|must only be called/i.test(message.text())) errors.push(message.text());
  });
  const waitFor = async words => {
    for (let i = 0; i < 400 && !logs.some(line => line.includes(words)); i++) await page.waitForTimeout(100);
    assert(logs.some(line => line.includes(words)), `missing ${words}: ${logs.slice(-10).join(' | ')}`);
  };
  await page.goto(`http://127.0.0.1:${server.address().port}${base}`);
  await waitFor('Day/night loaded');
  if (!process.env.SINDRI_DAYNIGHT_PREVIEW) {
    for (const phase of ['sunrise', 'midday', 'sunset', 'night']) {
      await waitFor(`Day/night ${phase}`);
    }
  }
  await waitFor('Day/night ready');
  const pixels = await page.screenshot(shot ? { path: shot } : {});
  assert(imageStatistics(pixels).colors >= 4, 'imported geometry renders');
  for (const asset of ['daynight.scene', 'daynight.decay', 'prop.glb', 'caster.glb']) {
    assert([...fetched].some(path => path.endsWith('/' + asset)), `fetched ${asset}`);
  }
  assert.deepEqual(errors, [], 'no runtime or GPU errors');
  console.log('Day/night: four assets fetched, scripted goal and rendered project passed');
} finally {
  if (browser) await browser.close();
  await new Promise(done => server.close(done));
}
