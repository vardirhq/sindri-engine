// Opens the browser Sprite Baker at desktop and phone sizes and uses it.
//
//   node scripts/browser/baker.mjs <site-dir> [capture-dir]
//
// <site-dir> holds index.html, app.js, baker/ and vendor/ (what
// tools/isometric-baker/web/build.mjs writes). It checks that every example in
// the catalogue bakes, that each frame is drawn on its floor, that the inspector
// edits the recipe text, that a model file opened or dropped on the page is read
// by Three.js and baked (GLB and OBJ), that a recipe waiting for its model says
// so and takes it when it arrives, that a broken recipe is a message, that the
// zip downloads, and that nothing scrolls sideways on a phone. The screenshots
// are kept for a person to look at.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, normalize } from 'node:path';
import assert from 'node:assert/strict';

// Node strips the types of the test helpers, which build the model files used here.
import { buildGlb, buildGltf, crateParts, texturedCrateParts } from '../../tools/isometric-baker/test/glb.ts';
import { encodePng } from '../../tools/isometric-baker/web/png.ts';
import { zip } from '../../tools/isometric-baker/web/zip.ts';

const [root, captures = 'target/baker-captures'] = process.argv.slice(2);
if (!root) throw new Error('usage: baker.mjs <site-dir> [capture-dir]');
await mkdir(captures, { recursive: true });
const scratch = await mkdtemp(join(tmpdir(), 'baker-smoke-'));

const types = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.glb': 'model/gltf-binary' };
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

// Files to open: a GLB made in the test, an OBJ, and a recipe that waits for its model.
const glb = join(scratch, 'my-prop.glb');
await writeFile(glb, buildGlb(crateParts()));
const waitingModel = join(scratch, 'waiting.glb');
await writeFile(waitingModel, buildGlb(crateParts()));
const waitingRecipe = join(scratch, 'waiting.isobake');
await writeFile(
  waitingRecipe,
  JSON.stringify({
    format_version: 1,
    id: 'waiting',
    texture: 'textures/waiting.png',
    tile: { width: 64, height: 32 },
    directions: 4,
    footprint: { width: 1, height: 1 },
    model: { kind: 'file', file: 'models/waiting.glb', auto_fit: true, fill: 0.9 },
  }),
);
// A Sketchfab-style download: a .gltf that points at scene.bin and a texture in textures/.
// The texture is four pixels, two red and two blue, and the crate's top looks up the red.
const sketch = join(scratch, 'sketchfab');
const { gltf, bin } = buildGltf(texturedCrateParts(), { textured: true });
const texture = encodePng({
  width: 4,
  height: 1,
  data: Uint8ClampedArray.from([200, 40, 40, 255, 200, 40, 40, 255, 40, 40, 200, 255, 40, 40, 200, 255]),
});
await mkdir(join(sketch, 'textures'), { recursive: true });
await writeFile(join(sketch, 'scene.gltf'), gltf);
await writeFile(join(sketch, 'scene.bin'), bin);
await writeFile(join(sketch, 'textures', 'body.png'), texture);
const archive = join(scratch, 'blacksmith.zip');
await writeFile(
  archive,
  zip([
    { path: 'scene.gltf', contents: new TextEncoder().encode(gltf) },
    { path: 'scene.bin', contents: bin },
    { path: 'textures/body.png', contents: texture },
    { path: 'license.txt', contents: new TextEncoder().encode('CC-BY-4.0') },
  ]),
);
const wedge = 'v 0 0 0\nv 1 0 0\nv 0 0 1\nv 0 1 0\nf 1 2 3\nf 1 3 4\nf 1 4 2\nf 2 4 3\n';

const sizes = [
  { name: 'desktop', viewport: { width: 1440, height: 900 } },
  { name: 'phone', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true },
];

// CHROMIUM lets a machine with its own Chromium skip Playwright's download.
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || undefined });
try {
  for (const { name, ...device } of sizes) {
    const phone = name === 'phone';
    const context = await browser.newContext({ ...device, acceptDownloads: true });
    const page = await context.newPage();
    const problems = [];
    page.on('pageerror', error => problems.push(String(error)));
    page.on('console', message => {
      if (message.type() === 'error') problems.push(message.text());
    });

    const ok = () =>
      page.waitForFunction(() => document.querySelector('#status')?.classList.contains('ok'), null, { timeout: 30000 });
    const failed = () =>
      page.waitForFunction(() => document.querySelector('#status')?.classList.contains('error'), null, { timeout: 10000 });
    // On a phone one pane shows at a time; on a desktop these are no-ops.
    const pane = async which => { if (phone) await page.click(`#panes button[data-show=${which}]`); };
    const tab = async which => page.click(`#tab-${which}`);
    const recipeText = () => page.evaluate(() => document.querySelector('#recipe').value);
    const status = () => page.textContent('#status');
    const assetNames = () => page.locator('#assets .asset span').allTextContents();
    /** Choose a catalogue entry by position or by name, and wait for its bake. */
    const pick = async which => {
      await pane('assets');
      const entry = typeof which === 'number' ? page.locator('#assets .asset').nth(which) : page.locator('#assets .asset', { hasText: which }).first();
      await entry.click();
      await ok();
    };

    await page.goto(url);
    await ok();

    // The catalogue lists every example and each one bakes, in every view the baker has.
    await pane('assets');
    const examples = await page.locator('#assets .asset').count();
    assert.ok(examples >= 6, `${name}: ${examples} examples in the catalogue`);
    for (let index = 0; index < examples; index++) {
      await pick(index);
      assert.ok((await page.locator('#cards canvas').count()) > 0, `${name}: example ${index} drew no frames`);
      await pane('assets');
      assert.equal(await page.locator('#assets .asset[aria-current=true]').count(), 1, `${name}: one asset is current`);
    }
    // The captures a person looks at, taken while the catalogue is still just the examples.
    await pick('barrel');
    await pane('preview');
    await page.screenshot({ path: join(captures, `baker-${name}.png`) });
    if (phone) {
      await pane('inspector');
      await tab('model');
      await page.screenshot({ path: join(captures, `baker-${name}-inspector.png`) });
      await tab('render');
      await pane('assets');
      await page.screenshot({ path: join(captures, `baker-${name}-catalogue.png`) });
    }
    // The crate and barrel are model files: Three.js read them and they baked.
    assert.ok((await assetNames()).includes('crate'), `${name}: the crate example is listed`);
    await pick('crate');
    await pane('assets');
    assert.match(await page.locator('#assets .asset[aria-current=true] small').textContent(), /model file/, `${name}: it is a model file`);

    // Each frame is drawn on a floor: more than the sprite's own pixels, in more than one color.
    await pane('preview');
    const cards = await page.evaluate(() =>
      [...document.querySelectorAll('#cards canvas')].map((card) => {
        const data = card.getContext('2d').getImageData(0, 0, card.width, card.height).data;
        const colors = new Set();
        for (let i = 0; i < data.length; i += 4 * 13) colors.add((data[i] << 16) | (data[i + 1] << 8) | data[i + 2] | (data[i + 3] << 24));
        return { width: card.width, colors: colors.size };
      }),
    );
    assert.equal(cards.length, 4, `${name}: one card per direction`);
    assert.ok(cards.every((card) => card.colors > 8), `${name}: a card is only a sprite, with no floor`);

    // Zoom resizes the cards; the floor toggle changes what is drawn without baking again.
    await page.locator('#zoom').fill('4');
    const zoomed = await page.evaluate(() => document.querySelector('#cards canvas').width);
    assert.ok(zoomed > cards[0].width, `${name}: zoom did not enlarge the cards`);
    await page.locator('#grid').uncheck();
    assert.ok((await page.evaluate(() => document.querySelector('#cards canvas').width)) < zoomed, `${name}: the floor toggle`);
    await page.locator('#grid').check();
    await page.locator('#zoom').fill(phone ? '2' : '3');

    // The inspector edits the recipe text, and bakes it again.
    await pane('inspector');
    await tab('render');
    await page.locator('[data-setting="supersample"]').fill('2');
    await page.waitForFunction(() => /"supersample":\s*2\b/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await page.locator('[data-setting="azimuth"]').fill('200');
    await page.waitForFunction(() => /"light"/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await ok();
    assert.equal(await page.locator('[data-out="azimuth"]').textContent(), '200°', `${name}: the light slider's label`);

    // Tuning a model file: fit, turn, and recolor one of its materials by name.
    await tab('model');
    assert.equal(await page.locator('#file-controls').isVisible(), true, `${name}: a model file has model controls`);
    await page.locator('[data-setting="fill"]').fill('0.5');
    await page.waitForFunction(() => /"fill":\s*0\.5\b/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await page.locator('[data-setting="rotation_y"]').fill('45');
    await page.waitForFunction(() => /"rotation_y":\s*45\b/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await page.locator('[data-setting="directions"]').selectOption('2');
    await page.waitForFunction(() => /"directions":\s*2\b/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await ok();
    assert.deepEqual(await page.locator('#materials .material span').allTextContents(), ['Wood', 'Metal'], `${name}: the file's materials`);
    await page.locator('#materials .material input[type=color]').first().fill('#336699');
    await page.waitForFunction(() => /"Wood":\s*"#336699"/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await ok();
    assert.equal(await page.locator('#materials .material button').first().isDisabled(), false, `${name}: the override can be reset`);
    await page.locator('#materials .material button').first().click();
    await page.waitForFunction(() => !/"Wood"/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await ok();
    // Footprint and facing apply to any recipe.
    await page.locator('[data-setting="facing"]').selectOption('east');
    await page.waitForFunction(() => /"facing":\s*"east"/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await ok();

    // Opening a model file makes it an asset of its own, fitted to a tile, ready to download.
    const before = (await assetNames()).length;
    await page.setInputFiles('#open', glb);
    await ok();
    await pane('assets');
    assert.equal((await assetNames()).length, before + 1, `${name}: the model joined the catalogue`);
    assert.equal(await page.locator('#assets .asset[aria-current=true] span').textContent(), 'my-prop', `${name}: named for its file`);
    assert.equal(await page.locator('#cards canvas').count(), 4, `${name}: baked in four directions`);
    assert.equal(await page.locator('#files li').count(), 3, `${name}: a PNG, a sheet and a prefab`);

    // Dropping a file does the same, whatever its format: here an .obj.
    await page.evaluate((text) => {
      const transfer = new DataTransfer();
      transfer.items.add(new File([text], 'wedge.obj'));
      window.dispatchEvent(new DragEvent('drop', { dataTransfer: transfer, bubbles: true, cancelable: true }));
    }, wedge);
    await page.waitForFunction(() => document.querySelector('#assets .asset[aria-current=true] span')?.textContent === 'wedge', null, { timeout: 15000 });
    await ok();
    await pane('assets');

    // A .gltf on its own cannot work: it says what it is missing, and the message is not wiped by the old bake.
    const countBefore = (await assetNames()).length;
    await page.setInputFiles('#open', join(sketch, 'scene.gltf'));
    await failed();
    assert.match(await status(), /scene\.gltf needs scene\.bin/, `${name}: says which file to add`);
    await page.waitForTimeout(500);
    assert.match(await page.locator('#status').getAttribute('class'), /error/, `${name}: the message stays on screen`);
    assert.equal((await assetNames()).length, countBefore, `${name}: nothing was added`);

    // A zip of the whole download just opens: textures give the model its colors.
    await page.setInputFiles('#open', archive);
    await ok();
    await pane('assets');
    assert.equal(await page.locator('#assets .asset[aria-current=true] span').textContent(), 'scene', `${name}: the zip's model is open`);
    assert.equal((await assetNames()).length, countBefore + 1, `${name}: one asset from the zip`);
    await pane('inspector');
    await tab('model');
    const swatches = await page.locator('#materials .material input[type=color]').evaluateAll((nodes) => nodes.map((node) => node.value));
    assert.deepEqual(swatches.sort(), ['#2828c8', '#c82828'], `${name}: the texture's red and blue became two colors`);
    assert.deepEqual(
      (await page.locator('#materials .material span').allTextContents()).sort(),
      ['Body 1', 'Body 2'],
      `${name}: named for the material they came from`,
    );
    await page.locator('[data-setting="texture_colors"]').fill('1');
    await page.waitForFunction(() => /"texture_colors":\s*1\b/.test(document.querySelector('#recipe').value), null, { timeout: 5000 });
    await ok();
    assert.deepEqual(await page.locator('#materials .material span').allTextContents(), ['Body'], `${name}: one color, the material's own name`);

    // The same folder, opened as a folder, and the .gltf with only its .bin chosen beside it.
    await page.setInputFiles('#open-folder', sketch);
    await ok();
    await pane('assets');
    assert.equal((await assetNames()).length, countBefore + 2, `${name}: a folder opens too`);
    await page.setInputFiles('#open', [join(sketch, 'scene.gltf'), join(sketch, 'scene.bin')]);
    await ok();
    await pane('assets');
    assert.equal((await assetNames()).length, countBefore + 3, `${name}: a .gltf and its .bin, chosen together`);

    // A file that is neither a model nor a recipe is a message, and a folder's license file is not.
    // A recipe whose model has not arrived says so, and takes it when it does.
    await page.setInputFiles('#open', waitingRecipe);
    await failed();
    assert.match(await status(), /needs the model file waiting\.glb/, `${name}: says what it is waiting for`);
    assert.equal(await page.locator('#download-all').isDisabled(), true, `${name}: nothing to download yet`);
    await pane('assets');
    const withRecipe = (await assetNames()).length;
    await page.setInputFiles('#open', waitingModel);
    await ok();
    await pane('assets');
    assert.equal((await assetNames()).length, withRecipe, `${name}: the model went to the recipe that was waiting`);

    // A file that is neither a model nor a recipe is a message, not a crash.
    const notes = join(scratch, 'notes.txt');
    await writeFile(notes, 'hello');
    await page.setInputFiles('#open', notes);
    await failed();
    assert.match(await status(), /notes\.txt: not a model/, `${name}: names the file it cannot use`);
    await pick('crate');

    // The zip comes back, named for the asset.
    const [download] = await Promise.all([page.waitForEvent('download'), page.click('#download-all')]);
    assert.match(download.suggestedFilename(), /\.zip$/, `${name}: the zip is named for the asset`);

    // A recipe that does not parse is a message, the inspector steps aside, and the last good frames stay.
    await pane('inspector');
    await tab('recipe');
    await page.fill('#recipe', '{ "format_version": 1, ');
    await failed();
    assert.ok(await page.locator('#download-all').isDisabled(), `${name}: nothing to download from a broken recipe`);
    assert.ok(await page.locator('#controls').evaluate((node) => node.disabled), `${name}: sliders cannot edit text that is not JSON`);
    await pick(0);

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    assert.ok(overflow <= 0, `${name}: the page scrolls sideways by ${overflow}px`);

    assert.deepEqual(problems, [], `${name}: the page reported errors`);
    console.log(`baker ${name}: ${examples} examples, model files read, no errors`);
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}
