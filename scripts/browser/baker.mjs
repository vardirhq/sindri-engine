// Opens the browser Isometric Baker at desktop and phone sizes and bakes in it.
//
//   node scripts/browser/baker.mjs <site-dir> [capture-dir]
//
// <site-dir> holds index.html, app.js and baker/ (tools/isometric-baker/web/build.mjs).
// It checks that the page bakes its first example without an error, draws a
// sheet, hands back a zip, reports a broken recipe instead of throwing, and
// does not scroll sideways on a phone. The screenshots are kept for a person.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { mkdir, readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import assert from 'node:assert/strict';

const [root, captures = 'target/baker-captures'] = process.argv.slice(2);
if (!root) throw new Error('usage: baker.mjs <site-dir> [capture-dir]');
await mkdir(captures, { recursive: true });

const types = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json' };
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
  { name: 'phone', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true },
];

// CHROMIUM lets a machine with its own Chromium skip Playwright's download.
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || undefined });
try {
  for (const { name, ...device } of sizes) {
    const context = await browser.newContext({ ...device, acceptDownloads: true });
    const page = await context.newPage();
    const problems = [];
    page.on('pageerror', error => problems.push(String(error)));
    page.on('console', message => {
      if (message.type() === 'error') problems.push(message.text());
    });
    await page.goto(url);
    await page.waitForFunction(() => document.querySelector('#status')?.classList.contains('ok'), null, { timeout: 30000 });

    const drawn = await page.evaluate(() => {
      const canvas = document.querySelector('#sheet');
      const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
      let opaque = 0;
      for (let i = 3; i < pixels.length; i += 4) if (pixels[i] > 0) opaque++;
      return { width: canvas.width, opaque };
    });
    assert.ok(drawn.width > 1 && drawn.opaque > 100, `${name}: the sheet drew ${drawn.opaque} pixels`);
    assert.equal(await page.locator('#files li').count(), 3, `${name}: a bake lists a PNG, a sheet and a prefab`);

    const [download] = await Promise.all([page.waitForEvent('download'), page.click('#download-all')]);
    assert.match(download.suggestedFilename(), /\.zip$/, `${name}: the zip is named for the asset`);

    // A recipe that does not parse is a message, not a crash, and the last good sheet stays.
    await page.fill('#recipe', '{ "format_version": 1, ');
    await page.waitForFunction(() => document.querySelector('#status')?.classList.contains('error'), null, { timeout: 5000 });
    assert.ok(await page.locator('#download-all').isDisabled(), `${name}: nothing to download from a broken recipe`);
    await page.selectOption('#example', { index: 1 });
    await page.waitForFunction(() => document.querySelector('#status')?.classList.contains('ok'), null, { timeout: 30000 });

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    assert.ok(overflow <= 0, `${name}: the page scrolls sideways by ${overflow}px`);

    await page.selectOption('#example', { index: 0 });
    await page.waitForFunction(() => document.querySelector('#status')?.classList.contains('ok'), null, { timeout: 30000 });
    await page.screenshot({ path: join(captures, `baker-${name}.png`) });
    assert.deepEqual(problems, [], `${name}: the page reported errors`);
    console.log(`baker ${name}: baked, drew ${drawn.opaque} pixels, no errors`);
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}
