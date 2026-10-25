// Opens the browser Sound Lab at desktop and phone sizes, interacts, and checks it draws.
//
//   node scripts/browser/sound-lab.mjs <site-dir> [capture-dir]
//
// <site-dir> holds index.html and pkg/ (the wasm-pack build). The Lab is one
// egui canvas, so the check is that it starts without an error, removes its
// loading note, responds to R in each mode, and paints something other than
// its background at each size; the screenshots are kept for a person to look at.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { mkdir, readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import assert from 'node:assert/strict';

const [root, captures = 'target/sound-lab-captures'] = process.argv.slice(2);
if (!root) throw new Error('usage: sound-lab.mjs <site-dir> [capture-dir]');
await mkdir(captures, { recursive: true });

const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');
  try {
    const body = await readFile(join(root, path.endsWith('/') ? `${path}index.html` : path));
    response.writeHead(200, { 'content-type': types[extname(path)] ?? 'text/html' });
    response.end(body);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;

const sizes = [
  { name: 'desktop', viewport: { width: 1440, height: 900 } },
  { name: 'desktop-sfx', query: '?mode=sfx', viewport: { width: 1440, height: 900 } },
  // A phone's width and touch, at pixel ratio 1: Chromium's emulated ratio is
  // not applied to `devicePixelContentBoxSize`, which eframe sizes its canvas
  // from, so an emulated 3x phone draws a third of its width. Real phones
  // report device pixels there.
  { name: 'phone', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true },
  { name: 'phone-sfx', query: '?mode=sfx', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true },
];

// CHROMIUM lets a machine with its own Chromium skip Playwright's download.
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM || undefined,
  args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'],
});
try {
  for (const size of sizes) {
    const { name, query = '', ...device } = size;
    const context = await browser.newContext(device);
    const page = await context.newPage();
    const problems = [];
    page.on('pageerror', error => problems.push(String(error)));
    page.on('console', message => {
      if (message.type() === 'error') problems.push(message.text());
    });
    await page.goto(url + query);
    await page.waitForSelector('#loading', { state: 'detached', timeout: 60000 });
    await page.waitForTimeout(1500);
    // V varies the selected lead; R regenerates the selected part or creates
    // a new SFX variation.
    await page.locator('#sound-lab').click({ position: { x: 12, y: 12 } });
    if (!name.endsWith('sfx')) {
      await page.keyboard.press('v');
      await page.waitForTimeout(250);
    }
    await page.keyboard.press('r');
    await page.waitForTimeout(250);
    const shot = await page.screenshot({ path: join(captures, `sound-lab-${name}.png`) });
    // A frame of only the page background means egui never painted.
    const painted = await page.evaluate(async png => {
      const image = await createImageBitmap(new Blob([new Uint8Array(png)], { type: 'image/png' }));
      const canvas = new OffscreenCanvas(image.width, image.height);
      const draw = canvas.getContext('2d');
      draw.drawImage(image, 0, 0);
      const pixels = draw.getImageData(0, 0, image.width, image.height).data;
      const colors = new Set();
      for (let i = 0; i < pixels.length; i += 4 * 97) colors.add(pixels[i] << 16 | pixels[i + 1] << 8 | pixels[i + 2]);
      return colors.size;
    }, [...shot]);
    assert.ok(painted > 20, `${name}: the Lab painted ${painted} colours`);
    assert.deepEqual(problems, [], `${name}: the page reported errors`);
    console.log(`sound lab ${name}: drew ${painted} colours, no errors`);
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}
