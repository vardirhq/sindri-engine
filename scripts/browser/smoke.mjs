// Loads a wasm-pack build in a real browser and reports what happened.
//
// Compiling wasm proves almost nothing about delivery. This check insists on a
// configured canvas, settled audio promises, and — when SINDRI_EXPECT_ASSETS is
// set — real HTTP requests for project assets. Projects can narrow that check
// with SINDRI_EXPECT_ASSET_KINDS; otherwise every manifest kind is required.
// It can also deliberately remove a browser capability to prove the page fails
// in a way a player can actually read.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { imageStatistics } from './png.mjs';
import { physicsDemo } from './physics-demo.mjs';
import { freshUiEvidence, uiDemo, uiDemoEvidence } from './ui-demo.mjs';
import { platformerGoal, platformerState } from './platformer-goal.mjs';

const ROOT = resolve(process.argv[2] ?? 'examples/cube');
const SHOT = process.argv[3];
const EXPECT_ASSETS = process.env.SINDRI_EXPECT_ASSETS === '1';
const EXPECT_ASSET_KINDS = new Set(
  (process.env.SINDRI_EXPECT_ASSET_KINDS ?? '')
    .split(',')
    .map((kind) => kind.trim())
    .filter(Boolean),
);
const EXPECT_FAILURE = process.env.SINDRI_EXPECT_FAILURE || '';
const VIEWPORT = {
  width: Number(process.env.SINDRI_VIEWPORT_WIDTH ?? 960),
  height: Number(process.env.SINDRI_VIEWPORT_HEIGHT ?? 540),
};
if (
  !Number.isInteger(VIEWPORT.width) ||
  !Number.isInteger(VIEWPORT.height) ||
  VIEWPORT.width <= 0 ||
  VIEWPORT.height <= 0
) {
  throw new Error('SINDRI_VIEWPORT_WIDTH and SINDRI_VIEWPORT_HEIGHT must be positive integers');
}
let BASE = process.env.SINDRI_BASE_PATH || '/';
if (!BASE.startsWith('/')) BASE = `/${BASE}`;
if (!BASE.endsWith('/')) BASE += '/';

const TYPES = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.ttf': 'font/ttf',
  '.wav': 'audio/wav',
  '.ogg': 'audio/ogg',
  '.mp3': 'audio/mpeg',
};

const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
  if (path === '/favicon.ico' || path === `${BASE}favicon.ico`) {
    response.writeHead(204).end();
    return;
  }
  if (!path.startsWith(BASE)) {
    response.writeHead(404).end('outside base path');
    return;
  }
  const relative = path.slice(BASE.length);
  const file = join(ROOT, relative === '' ? 'index.html' : relative);
  try {
    let body = await readFile(file);
    if (EXPECT_FAILURE === 'canvas' && relative === '') {
      body = Buffer.from(body.toString().replace('id="sindri-canvas"', 'id="wrong-canvas"'));
    }
    response.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' });
    response.end(body);
  } catch {
    response.writeHead(404).end('not found');
  }
});
await new Promise((done) => server.listen(0, done));
const port = server.address().port;

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || undefined,
  args: [
    '--enable-unsafe-webgpu',
    '--enable-features=Vulkan',
    '--use-angle=vulkan',
    '--use-vulkan=swiftshader',
    '--enable-gpu',
    '--ignore-gpu-blocklist',
    '--no-sandbox',
  ],
});
const page = await browser.newPage({ viewport: VIEWPORT, hasTouch: VIEWPORT.width < 600 });

// The interface existing is not the same as WebGPU working. Chrome on Android
// exposes `navigator.gpu` more widely than its drivers can serve, and a page
// that only checks for the interface starts anyway and fails where nobody can
// see it — which is what a blank canvas on a phone turned out to be.
if (EXPECT_FAILURE === 'adapter') {
  await page.addInitScript(() => {
    GPU.prototype.requestAdapter = () => Promise.resolve(null);
  });
}

// The one failure the page cannot catch for itself. An adapter exists, so the
// checks pass and `init()` resolves — and then the device request fails inside
// the event loop winit has already handed to the page, with nobody to return
// it to. This is the case that proves the engine's failure event reaches the
// page, rather than the failure living in a console no player opens.
if (EXPECT_FAILURE === 'device') {
  await page.addInitScript(() => {
    GPUAdapter.prototype.requestDevice = () =>
      Promise.reject(new Error('the device request was refused for this test'));
  });
}

await page.addInitScript(() => {
  window.__plays = [];
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () {
    const record = { settled: 'pending' };
    window.__plays.push(record);
    const element = this;
    return play.call(this).then(
      () => {
        record.settled = 'played';
        record.element = element;
      },
      (error) => {
        record.settled = `refused: ${error.message}`;
      },
    );
  };
});

const problems = [];
const tweenActions = new Set();
let cameraChanges = 0;
let cameraImpacts = 0;
const physicsEvidence = { results: new Set(), masks: new Set(), changes: 0, sensor: false, contact: false };
const uiEvidence = freshUiEvidence();
const spatialResults = new Set();
let spatialChanges = 0;
const inputEvidence = { boosts: 0, waiting: false, rebound: '' };
const mixerSaid = [];
const fetchedAssets = new Set();
const platformerEvidence = { latest: null };
const voxelEvidence = { dropped: false, landed: false };
// A shader that fails to compile is reported by the GPU implementation as a
// console *warning*, not an error, and the page carries on and presents empty
// frames. Collecting only errors is how a build whose every shape and glyph
// pipeline was rejected still reported that the engine ran.
const GPU_REJECTION = /error while parsing wgsl|is invalid|must only be called/i;
page.on('console', (message) => {
  const text = message.text();
  if (process.env.SINDRI_PLATFORMER_GOAL === '1') platformerState(platformerEvidence, text);
  uiDemoEvidence(uiEvidence, text);
  const ray = text.match(/Physics ray (.+)$/);
  if (ray) physicsEvidence.results.add(ray[1]);
  const mask = text.match(/Physics controls (ALL|SOLID|BODIES|NONE)/);
  if (mask) { physicsEvidence.masks.add(mask[1]); physicsEvidence.changes += 1; }
  if (text.includes('Physics sensor entered')) physicsEvidence.sensor = true;
  if (text.includes('Physics contact')) physicsEvidence.contact = true;
  const mixer = text.match(/Mixer (.+)$/);
  if (mixer) mixerSaid.push(mixer[1]);
  if (text.includes('Input boost')) inputEvidence.boosts += 1;
  if (text.includes('Input waiting')) inputEvidence.waiting = true;
  const rebound = text.match(/Input rebound (.+)$/);
  if (rebound) inputEvidence.rebound = rebound[1];
  if (text.includes('Spatial demo controls changed')) spatialChanges += 1;
  const result = text.match(/Spatial demo nearest (.+)/);
  if (result) spatialResults.add(result[1]);
  if (text.includes('Camera demo modes changed')) cameraChanges += 1;
  if (text.includes('Camera demo impact')) cameraImpacts += 1;
  if (text.includes('Voxel landing dropped')) voxelEvidence.dropped = true;
  if (text.includes('Voxel collision landing verified')) voxelEvidence.landed = true;
  const tweenAction = text.match(/Tween demo action ([1-5])/);
  if (tweenAction) tweenActions.add(Number(tweenAction[1]));
  if (message.type() === 'error') problems.push(text);
  else if (GPU_REJECTION.test(text)) problems.push(text.split('\n')[0]);
});
page.on('pageerror', (error) => problems.push(String(error.message)));
page.on('response', (response) => {
  if (!response.ok()) {
    problems.push(`HTTP ${response.status()} ${new URL(response.url()).pathname}`);
  }
});
page.on('request', (request) => {
  const path = new URL(request.url()).pathname;
  const marker = `${BASE}assets/`;
  if (path.startsWith(marker)) fetchedAssets.add(path.slice(marker.length));
});

await page.goto(`http://127.0.0.1:${port}${BASE}`, { waitUntil: 'load' });

if (EXPECT_FAILURE) {
  await page.waitForSelector('#sindri-error[data-visible="true"]');
  const message = await page.locator('#sindri-error').innerText();
  const expected = {
    canvas: 'missing the #sindri-canvas',
    adapter: 'WebGPU is unavailable',
    // Not the message's wording, which belongs to wgpu, and not the project's
    // name, which belongs to whichever project this was pointed at: what
    // matters is that a failure raised after startup finished arrived on the
    // page at all. Naming the game here meant renaming the game broke this.
    device: 'stopped',
  }[EXPECT_FAILURE];
  if (SHOT) await page.screenshot({ path: SHOT });
  await browser.close();
  server.close();
  console.log(`expected startup failure: ${message.replaceAll('\n', ' ')}`);
  if (!message.includes(expected)) {
    console.log(`problem: expected the failure UI to mention '${expected}'`);
    process.exit(1);
  }
  console.log('the page surfaced the expected startup failure');
  process.exit(0);
}

const webgpu = await page.evaluate(() => Boolean(navigator.gpu));

// Wait for the engine to have started rather than for a number of seconds to
// have passed. Starting means sizing its own canvas, which is exactly what
// `started` is checked on below, so this waits on the condition the run is
// about to be judged by.
//
// It used to be a flat six seconds. That is a bet on how fast the machine is,
// and on a loaded runner -- this job builds a wasm bundle and exports three
// projects first -- the bet loses: the canvas is still at its 300x150 default
// when it is measured, and the failure reads "the page did not start the
// engine" about an engine that was starting perfectly well. A timeout here is
// deliberately not an error: it falls through to the same checks and the same
// diagnostics, so an engine that genuinely never starts fails exactly as it
// did, just later.
await page
  .waitForFunction(
    () => {
      const canvas = document.querySelector('canvas');
      return Boolean(canvas) && canvas.width > 300;
    },
    { timeout: 60000 },
  )
  .catch(() => {});
// A page with a loading screen covers the canvas until the host announces
// the game is on screen. Wait for it to give way before touching or reading
// the canvas: a click on the loading screen reaches nothing, and a capture of
// it measures the loading screen rather than the game. Bounded, and not an
// error here, so a screen that never gives way fails below with its reason.
const hasLoadingScreen = await readFile(join(ROOT, 'index.html'), 'utf8')
  .then((page) => page.includes('id="sindri-loading"'))
  .catch(() => false);
if (hasLoadingScreen) {
  await page
    .waitForFunction(() => !document.querySelector('#sindri-loading'), null, { timeout: 60000 })
    .catch(() => {});
}
const loadingStuck =
  hasLoadingScreen && (await page.evaluate(() => Boolean(document.querySelector('#sindri-loading'))));
// Then a moment to draw a frame or two into the canvas it just sized.
await page.waitForTimeout(1500);

if (process.env.SINDRI_PLATFORMER_GOAL === '1') {
  await platformerGoal(page, ROOT, platformerEvidence, problems);
}

if (process.env.SINDRI_VOXEL_LANDING === '1') {
  // Causeway's own loose block, dropped by a read-only observer onto the
  // generated voxel terrain: it must be stopped by the world's blocks (and
  // reported by a downward ray) rather than returned by the give-up timer.
  for (let waited = 0; waited < 10000 && !voxelEvidence.landed; waited += 250) {
    await page.waitForTimeout(250);
  }
  if (!voxelEvidence.dropped) problems.push('voxel landing observer never dropped its block');
  if (!voxelEvidence.landed) problems.push('dropped block never landed on voxel collision');
}

if (process.env.SINDRI_PHYSICS_DEMO === '1') {
  await physicsDemo(page, VIEWPORT, physicsEvidence, problems);
}

if (process.env.SINDRI_UI_DEMO === '1') {
  await uiDemo(page, VIEWPORT, uiEvidence, problems);
}

if (process.env.SINDRI_TWEEN_DEMO === '1') {
  // Exercise Decay gameplay controls and observe the actual WebGPU output.
  await page.mouse.click(VIEWPORT.width / 2, VIEWPORT.height / 2);
  await page.keyboard.press('KeyR');
  await page.waitForTimeout(250);
  const first = await page.locator('canvas').screenshot();
  await page.waitForTimeout(750);
  const moving = await page.locator('canvas').screenshot();
  if (first.equals(moving)) problems.push('tween demo did not animate after restart');
  await page.keyboard.press('KeyP');
  await page.waitForTimeout(750);
  const paused = await page.locator('canvas').screenshot();
  await page.waitForTimeout(750);
  if (!paused.equals(await page.locator('canvas').screenshot())) {
    problems.push('tween demo moved while paused');
  }
  await page.keyboard.press('Space');
  await page.waitForTimeout(250);
  await page.keyboard.press('KeyV');
  await page.waitForTimeout(250);
  await page.keyboard.press('KeyC');
  await page.waitForTimeout(750);
  for (const action of [1, 2, 3, 4, 5]) {
    if (!tweenActions.has(action)) problems.push('tween demo missed control ' + action);
  }
}

if (process.env.SINDRI_MIXER_DEMO === '1') {
  // A click on empty space unlocks audio and lets go of focus, so Tab brings
  // it back to master, the first control; the arrows then move its bus.
  await page.mouse.click(VIEWPORT.width / 2, VIEWPORT.height * 0.9);
  await page.keyboard.press('Tab');
  await page.waitForTimeout(250);
  for (const _ of [0, 1]) {
    await page.keyboard.press('ArrowLeft');
    await page.waitForTimeout(250);
  }
  if (!mixerSaid.includes('master is at 90%.')) {
    problems.push('mixer did not move the master bus: ' + JSON.stringify(mixerSaid));
  }
}

if (process.env.SINDRI_INPUT_DEMO === '1') {
  // Boost by the declared action, rebind it to J through the page, and boost
  // again by the new key: the action layer, end to end in a browser.
  await page.mouse.click(VIEWPORT.width / 2, VIEWPORT.height / 2);
  const tap = async (key) => {
    await page.keyboard.down(key);
    await page.waitForTimeout(250);
    await page.keyboard.up(key);
    await page.waitForTimeout(250);
  };
  await tap('Space');
  if (inputEvidence.boosts < 1) problems.push('input demo did not boost on Space');
  await tap('KeyK');
  if (!inputEvidence.waiting) problems.push('input demo did not wait for a key');
  await tap('KeyJ');
  if (inputEvidence.rebound !== 'key.J') {
    problems.push('input demo rebound boost to ' + JSON.stringify(inputEvidence.rebound));
  }
  const before = inputEvidence.boosts;
  await tap('KeyJ');
  if (inputEvidence.boosts <= before) problems.push('input demo did not boost on the new key');
}

if (process.env.SINDRI_CAMERA_DEMO === '1') {
  const span = Math.min(1, VIEWPORT.width / VIEWPORT.height);
  const click = async (x, y) => {
    const px = VIEWPORT.width / 2 + x * span * VIEWPORT.height / 2;
    const py = (1 - y) * VIEWPORT.height / 2;
    if (VIEWPORT.width < 600) await page.touchscreen.tap(px, py);
    else await page.mouse.click(px, py);
    await page.waitForTimeout(300);
  };
  // The automatic tour makes movement accessible without a keyboard.
  await click(-0.31, 0.48);
  const first = await page.locator('canvas').screenshot();
  await page.waitForTimeout(1000);
  if (first.equals(await page.locator('canvas').screenshot())) {
    problems.push('camera tour did not move the rendered scene');
  }
  for (const [x, y] of [[-0.6, -0.49], [0, -0.49], [0.6, -0.49],
    [-0.6, -0.64], [0, -0.64], [0.6, -0.64]]) {
    await click(x, y);
  }
  if (cameraChanges < 7) problems.push('camera demo missed a screen control');
  await click(0.31, 0.48);
  if (cameraImpacts !== 1) problems.push('camera impact button did not reach Decay');
  await page.keyboard.press('KeyK');
  await page.waitForTimeout(300);
  await page.keyboard.press('Space');
  await page.waitForTimeout(300);
  if (cameraImpacts !== 1) problems.push('camera impact fired with shake disabled');
  await page.keyboard.press('KeyR');
  await page.waitForTimeout(2000);
}

if (process.env.SINDRI_SPATIAL_DEMO === '1') {
  const span = Math.min(1, VIEWPORT.width / VIEWPORT.height);
  const click = async (x, y) => {
    const px = VIEWPORT.width / 2 + x * span * VIEWPORT.height / 2;
    const py = (1 - y) * VIEWPORT.height / 2;
    if (VIEWPORT.width < 600) await page.touchscreen.tap(px, py);
    else await page.mouse.click(px, py);
    await page.waitForTimeout(300);
  };
  if (![...spatialResults].some(result => result.startsWith('A / 1:A  2:B  3:D  4:E'))) {
    problems.push('spatial demo did not show the expected stable world-space order');
  }
  await click(0, -0.54); // ally
  await click(0, -0.54); // missing
  if (!spatialResults.has('null / within_radius: []')) {
    problems.push('spatial demo did not expose an empty tag query');
  }
  await click(0, -0.54); // enemy
  for (const [x, y] of [[-0.6, -0.69], [0, -0.69], [0.6, -0.69],
    [-0.6, -0.54], [0.6, -0.54], [0, -0.84]]) {
    await click(x, y);
  }
  if (spatialChanges < 9) problems.push('spatial demo missed a screen control');
  if (![...spatialResults].some(result => result.startsWith('B / 1:B  2:E'))) {
    problems.push('spatial demo did not exclude the inactive target and parent');
  }
  const still = await page.locator('canvas').screenshot();
  await page.keyboard.down('ArrowRight');
  await page.waitForTimeout(600);
  await page.keyboard.up('ArrowRight');
  if (still.equals(await page.locator('canvas').screenshot())) {
    problems.push('spatial demo did not move its query origin');
  }
  await page.keyboard.press('KeyR');
  await page.waitForTimeout(300);
}

await page.mouse.click(480, 270);
await page.keyboard.press('KeyD');
await page.waitForTimeout(2000);
// A sound requested in that time may still be starting: `play()` settles once
// the element has loaded and begun, and on a software renderer at two or three
// frames a second that can land either side of any fixed moment. So wait for
// every requested sound to settle and, if it played, to have moved -- bounded,
// so one that never starts still fails, as it should.
await page
  .waitForFunction(
    () =>
      window.__plays.every(
        (record) =>
          record.settled !== 'pending' &&
          (record.settled !== 'played' || (record.element && record.element.currentTime > 0)),
      ),
    { timeout: 15000 },
  )
  .catch(() => {});
const audio = await page.evaluate(() =>
  window.__plays.map((record) => ({
    settled: record.settled,
    playedTo: record.element ? Number(record.element.currentTime.toFixed(2)) : 0,
  })),
);

const canvas = await page.evaluate(() => {
  const element = document.querySelector('canvas');
  return element ? { width: element.width, height: element.height } : null;
});

// What reached the canvas, rather than whether one exists. Every check above
// passes on a page that starts the engine and then presents nothing: a
// rejected pipeline draws no geometry and raises nothing the page can catch,
// so the only evidence left is the picture.
//
// Read from a screenshot rather than from the canvas itself, because a WebGPU
// canvas gives nothing back to `drawImage` -- the browser composites its
// contents but does not keep them readable. The screenshot is what a player
// sees, which is the thing in question anyway.
const drawn = canvas
  ? imageStatistics(await page.locator('canvas').screenshot())
  : null;

if (SHOT) await page.screenshot({ path: SHOT });
await browser.close();
server.close();

const started = Boolean(canvas) && canvas.width > 300;
const refused = audio.filter(
  (record) => record.settled !== 'played' || record.playedTo === 0,
);

// Read from the manifest the build actually shipped rather than hard-coding
// file names. By default every kind is required, which keeps the original
// broad delivery check. A focused demo can declare the kinds it is expected
// to exercise without failing merely because its project also contains an
// intentionally unused asset of some other kind.
const requiredAssetKinds = [['manifest', 'sindri.manifest']];
try {
  const manifest = JSON.parse(
    await readFile(join(ROOT, 'assets', 'sindri.manifest'), 'utf8'),
  );
  const first = new Map();
  for (const [id, entry] of Object.entries(manifest.assets ?? {})) {
    const kind = entry.kind ?? 'other';
    if (EXPECT_ASSET_KINDS.size > 0 && !EXPECT_ASSET_KINDS.has(kind)) continue;
    // The first of each kind in the manifest's own order, so the same build
    // asks for the same file every run.
    if (!first.has(kind)) first.set(kind, id);
  }
  for (const [kind, id] of first) requiredAssetKinds.push([kind, id]);

  if (EXPECT_ASSET_KINDS.size > 0) {
    for (const kind of EXPECT_ASSET_KINDS) {
      if (!first.has(kind)) {
        console.log(`problem: manifest has no ${kind} asset to verify`);
        process.exitCode = 1;
      }
    }
  }
} catch (error) {
  if (EXPECT_ASSETS) {
    console.log(`problem: no manifest to read asset kinds from: ${error.message}`);
    process.exitCode = 1;
  }
}
// Exported assets sit below a content-hash directory; the manifest itself
// deliberately does not. Match the logical ID at the end of either shape.
const fetched = (asset) =>
  [...fetchedAssets].some((path) => path === asset || path.endsWith(`/${asset}`));
const missingAssets = EXPECT_ASSETS
  ? requiredAssetKinds.filter(([, asset]) => !fetched(asset))
  : [];

// Deliberately loose. A game may legitimately be dark, and this is not a
// likeness test -- it separates a drawn frame from an empty one. The build
// that shipped every pipeline rejected measured 5 colors at a mean of 0.09;
// the same build working measures hundreds at a mean above 10.
const BLANK_COLORS = 16;
const BLANK_MEAN = 1;
const blank =
  drawn !== null && (drawn.colors < BLANK_COLORS || drawn.mean < BLANK_MEAN);

console.log(`webgpu: ${webgpu ? 'yes' : 'no'}`);
if (hasLoadingScreen) {
  console.log(`loading screen: ${loadingStuck ? 'still up' : 'gave way to the game'}`);
}
if (loadingStuck) {
  console.log('problem: the loading screen never gave way -- the host did not announce sindri:ready');
  process.exitCode = 1;
}
console.log(`canvas: ${canvas ? `${canvas.width}x${canvas.height}` : 'none'}`);
console.log(`base path: ${BASE}`);
console.log(
  audio.length === 0
    ? 'audio: none requested'
    : `audio: ${audio.length - refused.length}/${audio.length} playing`,
);
if (EXPECT_ASSETS) console.log(`assets fetched: ${fetchedAssets.size}`);
console.log(
  drawn === null
    ? 'drawn: no canvas to read'
    : `drawn: ${drawn.colors} colors, mean ${drawn.mean.toFixed(2)}`,
);
if (blank) {
  console.log(
    `problem: the canvas is blank (${drawn.colors} colors, mean ${drawn.mean.toFixed(2)}) -- the engine started but drew nothing`,
  );
}
for (const [kind, asset] of missingAssets) {
  console.log(`problem: no HTTP ${kind} request for assets/${asset}`);
}
for (const record of refused) console.log(`problem: audio ${record.settled}`);
for (const problem of problems) console.log(`problem: ${problem}`);
if (
  !webgpu ||
  !started ||
  blank ||
  problems.length > 0 ||
  refused.length > 0 ||
  missingAssets.length > 0 ||
  process.exitCode === 1
) {
  console.log('the page did not start the engine');
  process.exit(1);
}
console.log('the engine ran in a browser');
