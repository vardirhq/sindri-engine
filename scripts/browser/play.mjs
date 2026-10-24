// Check the landing page and the play pages through HTTP at desktop and phone
// sizes: every catalogued game has a page, the game loads only when asked,
// it plays inline or over the whole window as the "Full page" switch says,
// and Back leaves full page rather than the page.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve, join, extname, sep } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(process.argv[2] || 'target/pages');
const captures = resolve(process.argv[3] || 'target/directory-captures');
const entries = JSON.parse(await readFile('site/directory.json', 'utf8'));
const playable = entries.filter(entry => entry.play);
const route = entry => entry.path.split('/')[1];
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  if (url.pathname === '/favicon.ico') return response.writeHead(204).end();
  let path = resolve(root, '.' + decodeURIComponent(url.pathname));
  if (url.pathname.endsWith('/')) path = join(path, 'index.html');
  if (!path.startsWith(root + sep)) return response.writeHead(404).end();
  try {
    const body = await readFile(path);
    const mime = {
      '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript',
      '.png': 'image/png', '.jpg': 'image/jpeg', '.wasm': 'application/wasm',
    };
    response.writeHead(200, { 'Content-Type': mime[extname(path)] || 'application/octet-stream' }).end(body);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const base = 'http://127.0.0.1:' + server.address().port;
// Fonts come from Google; the check is about this site, so leave them out.
const fonts = /^https:\/\/fonts\.(googleapis|gstatic)\.com\//;
const browser = await chromium.launch({ headless: true });
await mkdir(captures, { recursive: true });

async function noOverflow(page, what) {
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true,
    `${what} scrolls sideways`,
  );
}

try {
  const desktop = { viewport: { width: 1280, height: 900 } };
  const phone = { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true };
  for (const [name, options] of [['desktop', desktop], ['phone', phone]]) {
    const context = await browser.newContext(options);
    await context.route(fonts, request => request.abort());
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => {
      if (response.url().startsWith(base) && response.status() >= 400) errors.push(response.url() + ' ' + response.status());
    });

    // The landing page lists every playable project, from the catalog.
    await page.goto(base + '/');
    for (const entry of playable) {
      assert.equal(await page.locator(`a[href="./play/${route(entry)}/"]`).count() > 0, true, `landing links ${entry.title}`);
    }
    await noOverflow(page, 'landing page');
    await page.screenshot({ path: join(captures, `landing-${name}.png`), fullPage: true });

    // Every play page exists and waits to be asked before loading its game.
    for (const entry of playable) {
      await page.goto(`${base}/play/${route(entry)}/`);
      assert.equal(await page.locator('h1').textContent(), entry.title);
      assert.equal(await page.locator('#stage iframe').count(), 0, `${entry.title} loads before Play`);
      await noOverflow(page, entry.title);
    }

    // Playing: inline on a desktop, full page on a phone, by default.
    await page.goto(`${base}/play/causeway/`);
    await page.screenshot({ path: join(captures, `play-${name}.png`), fullPage: true });
    const switchOn = name === 'phone';
    assert.equal(await page.locator('#fullpage-pref').isChecked(), switchOn);
    await page.locator('#launch').click();
    const frame = page.locator('#stage iframe');
    await frame.waitFor();
    assert.equal(new URL(await frame.getAttribute('src'), base + '/play/causeway/').pathname, '/examples/causeway/');
    const stage = page.locator('#stage');
    assert.equal(await stage.evaluate(node => node.classList.contains('full')), switchOn);
    if (!switchOn) await page.locator('.stage-actions [data-action="fullpage"]').click();
    const box = await stage.boundingBox();
    const size = options.viewport;
    assert.deepEqual([box.x, box.y, box.width, box.height], [0, 0, size.width, size.height], 'full page fills the window');
    assert.equal(await page.locator('.stage-controls [data-action="exit"]').isVisible(), true);
    await page.screenshot({ path: join(captures, `play-${name}-full.png`) });

    // Back leaves full page and stays on the page, with the game still there.
    await page.goBack();
    await page.waitForFunction(() => !document.querySelector('#stage').classList.contains('full'));
    assert.match(page.url(), /\/play\/causeway\/$/);
    assert.equal(await frame.count(), 1);

    // The exit button does the same, and the switch is remembered.
    await page.locator('.stage-actions [data-action="fullpage"]').click();
    await page.locator('.stage-controls [data-action="exit"]').click();
    assert.equal(await stage.evaluate(node => node.classList.contains('full')), false);
    await page.locator('.switch').click();
    await page.reload();
    assert.equal(await page.locator('#fullpage-pref').isChecked(), !switchOn, 'switch remembered');

    assert.deepEqual(errors, []);
    await context.close();
  }

  // Without JavaScript the page still says what the game is and links to it.
  const context = await browser.newContext({ javaScriptEnabled: false });
  await context.route(fonts, request => request.abort());
  const page = await context.newPage();
  await page.goto(`${base}/play/lamplighter/`);
  assert.equal(await page.locator('noscript a[href="../../examples/lamplighter/"]').count(), 1);
  await context.close();
  console.log(`landing page and ${playable.length} play pages passed on desktop and phone`);
} finally {
  await browser.close();
  await new Promise(done => server.close(done));
}
