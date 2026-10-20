// Real WebGPU player flows, including HiDPI touch and persisted Unicode text.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { chromium } from 'playwright';
import { imageStatistics } from './png.mjs';

const root = resolve(process.argv[2]);
const output = resolve(process.argv[3] || 'target/render-artifacts/orbital-complete');
const base = process.env.SINDRI_BASE_PATH || '/examples/orbital-baked/';
const saveKey = 'sindri.orbital.save';
const types = { '.html': 'text/html', '.js': 'application/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png', '.ttf': 'font/ttf', '.wav': 'audio/wav', '.ogg': 'audio/ogg' };
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
  if (path.endsWith('/favicon.ico')) { response.writeHead(204).end(); return; }
  if (!path.startsWith(base)) { response.writeHead(404).end(); return; }
  const file = join(root, path.slice(base.length) || 'index.html');
  try { const bytes = await readFile(file); response.writeHead(200, { 'Content-Type': types[extname(file)] || 'application/octet-stream' }).end(bytes); }
  catch { response.writeHead(404).end(); }
});
await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
const origin = `http://127.0.0.1:${server.address().port}`;
await mkdir(output, { recursive: true });
let browser;
try {
  for (const viewport of [{ width: 1280, height: 720 }, { width: 390, height: 844 }]) {
    const phone = viewport.width < viewport.height;
    const scale = phone ? 1.5 : 1;
    // Emulating a scale alone leaves the canvas's device-pixel size at CSS
    // pixels while `devicePixelRatio` scales the pointer, a pairing no real
    // browser reports. Forcing the same scale on the browser makes them agree.
    await browser?.close();
    browser = await chromium.launch({
      channel: 'chromium', executablePath: process.env.CHROME_PATH || undefined,
      args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--use-vulkan=swiftshader', '--enable-gpu', '--ignore-gpu-blocklist', '--no-sandbox', `--force-device-scale-factor=${scale}`],
    });
    const context = await browser.newContext({ viewport, hasTouch: phone, deviceScaleFactor: scale });
    const page = await context.newPage();
    const errors = [], logs = [], fetched = new Set();
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      logs.push(message.text());
      if (message.type() === 'error' || /error while parsing wgsl|is invalid|must only be called|script.*(?:failed|error)/i.test(message.text())) errors.push(message.text());
    });
    page.on('response', (response) => {
      if (response.status() >= 400) errors.push(`HTTP ${response.status()}: ${response.url()}`);
      fetched.add(new URL(response.url()).pathname.split('/').at(-1));
    });
    await page.addInitScript(({ key }) => {
      if (!localStorage.getItem(key)) localStorage.setItem(key, JSON.stringify({ version: 1, values: { credits: 100 } }));
      window.orbitalAudio = 0;
      const start = AudioBufferSourceNode.prototype.start;
      AudioBufferSourceNode.prototype.start = function (...args) { window.orbitalAudio++; return start.apply(this, args); };
    }, { key: saveKey });
    const ready = async () => {
      await page.waitForSelector('canvas');
      await page.waitForFunction(() => !document.querySelector('#sindri-loading'), null, { timeout: 60000 });
      await page.waitForTimeout(1200);
    };
    const value = async (name) => page.evaluate(({ key, name }) => JSON.parse(localStorage.getItem(key) || '{}').values?.[name], { key: saveKey, name });
    const saved = async (name, expected) => {
      const deadline = Date.now() + 60000;
      let actual;
      do {
        actual = await value(name);
        if (actual === expected) return;
        await page.waitForTimeout(250);
      } while (Date.now() < deadline);
      await page.screenshot({ path: join(output, 'failure.png'), timeout: 60000 });
      console.error({ name, expected, actual, errors, logs: logs.slice(-20) });
      assert.equal(actual, expected, `saved ${name}`);
    };
    const click = async (desktop, mobile) => {
      const [x, y] = phone ? mobile : desktop;
      if (phone) await page.touchscreen.tap(x, y); else await page.mouse.click(x, y);
      await page.waitForTimeout(400);
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    };
    const center = [viewport.width / 2, viewport.height / 2];
    const vh = viewport.height / 100, vw = viewport.width / 100;
    const at = (x, y) => [center[0] + x * vh, center[1] - y * vh];
    const mobile = (x, y) => [center[0] + x * vw, center[1] - y * vh];
    const shot = async (name) => {
      if (process.env.ORBITAL_NO_SHOTS) return;
      const pixels = await page.screenshot({ path: join(output, `${phone ? 'phone' : 'desktop'}-${name}.png`), timeout: 60000 });
      assert(imageStatistics(pixels).colors > 200, `${name} has a rendered scene and UI`);
    };
    await page.goto(origin + base);
    await ready();
    console.log(`${phone ? 'phone' : 'desktop'} ready`);
    await shot('title');
    // The real field and text-save API, then a real settings interaction.
    await click(at(-44, -32), mobile(0, 3));
    await page.waitForFunction(() => document.activeElement?.tagName === 'TEXTAREA', null, { polling: 100, timeout: 60000 });
    await page.keyboard.insertText('Nova-é7');
    await saved('callsign', 'Nova-é7');
    console.log('callsign persisted');
    // Software WebGPU can take over a second a frame. A click landing in the
    // same step as Escape opens a page that Escape then closes, so wait for
    // the step that let go of the field: it hands focus back to the canvas.
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => document.activeElement?.tagName !== 'TEXTAREA', null, { polling: 100, timeout: 60000 });
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    await click(at(56, -31), mobile(30.6, -39));
    await click(at(34, 16), mobile(0, 0));
    await saved('reduced_motion', true);
    // Move Music to 35%, through the pointer/touch slider hit area.
    await click(at(-24 - 21 + 42 * 0.35, 6), mobile(13 - 24.5 + 49 * 0.35, 16));
    await page.waitForFunction(({ key }) => Math.abs(JSON.parse(localStorage.getItem(key)).values.volume_music - 0.35) < 0.01, { key: saveKey }, { polling: 100, timeout: 60000 });
    await shot('settings');
    // Unlock a sidegrade from a known starting wallet. Equipping twice is free.
    await click(at(-54, 29), mobile(-33, 32));
    await click(at(-49, -29), mobile(-11, -5));
    await click(at(31, -27), mobile(0, -35.5));
    await saved('ship_1', true);
    await saved('selected_ship', 1);
    assert.equal(await value('credits'), 20);
    await shot('hangar');
    await click(at(0, -38), mobile(0, -41));
    await click(at(43, 24), mobile(0, -5));
    await page.waitForTimeout(1500);
    if (phone) await click([0, 0], [viewport.width - 150, viewport.height - 0.35 * viewport.width]);
    else await page.keyboard.press('Shift');
    await shot('combat');
    await click([viewport.width - 72, viewport.height - 44], [viewport.width - 50, viewport.height - 0.35 * viewport.width]);
    await shot('pause');
    // Settings opened from pause return to pause, with underlying controls hidden.
    await click(at(-31, 9), mobile(0, -40));
    await click(at(0, -38), mobile(22, -41));
    await click(at(-31, -11), mobile(0, 16.4));
    await page.waitForFunction(({ key }) => JSON.parse(localStorage.getItem(key)).values.record_0?.includes('BASTION'), { key: saveKey }, { polling: 100, timeout: 60000 });
    const record = await value('record_0');
    assert(record.includes('LOST'), 'abandoning settles a real lost sortie');
    await shot('debrief');
    await click(at(-31, -24), mobile(0, -36.4));
    await click(at(30, -31), mobile(10.2, -39));
    await shot('flight-log');
    // Counted before the reload: a fresh page has had no gesture to unlock audio.
    assert(await page.evaluate(() => window.orbitalAudio) >= 3, 'the composed score starts through Web Audio');
    // Opening a new session restores the same player and career, not a fixture UI.
    await page.reload();
    await ready();
    await saved('callsign', 'Nova-é7');
    assert.equal(await value('selected_ship'), 1);
    assert.equal(await value('record_0'), record);
    assert.equal(await value('reduced_motion'), true);
    for (const asset of ['orbital-key-art.png', 'fleet-1.png', 'flight-manual.profile', 'briefing.ogg', 'corridor.ogg', 'last-signal.ogg', 'burst.wav']) {
      assert(fetched.has(asset), `browser fetched ${asset}`);
    }
    assert.deepEqual(errors, [], `browser errors: ${errors.join(' | ')}`);
    console.log(`Orbital ${phone ? 'HiDPI touch' : 'desktop'}: pilot, settings, purchase/equip, burst, pause, debrief, flight log and reload passed`);
    await context.close();
  }
} finally {
  await browser?.close();
  await new Promise((done) => server.close(done));
}
