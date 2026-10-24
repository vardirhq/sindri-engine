// The Sprite Baker page: a catalogue of assets on the left, each baked frame in
// context in the middle, and the inspector on the right.
//
// Everything runs here. `baker/` is the tool's own TypeScript with its types
// stripped (see build.mjs), so a bake in this page is the bake the command line
// does, except for the PNG's compressed bytes: the pixels are identical. A model
// file is read into triangles by Three.js (models.js) and baked by that same code.
//
// The recipe text is the one source of truth. Selecting an asset puts its recipe
// in the editor; the inspector's controls read the recipe the last bake parsed
// and move by rewriting that text (controls.js).
import { bake } from './baker/bake.js';
import { RecipeError, parseRecipe } from './baker/recipe.js';
import { zip } from './baker/zip.js';
import { framePreview } from './context.js';
import { setupControls, setupTabs } from './controls.js';
import { extensionOf, isModelFile, readModelFile } from './models.js';

const $ = (id) => document.getElementById(id);
const recipeBox = $('recipe');
const status = $('status');
const stage = $('stage');
const canvas = $('sheet');
const zoom = $('zoom');
const grid = $('grid');

/** Every asset in the catalogue: `{ name, recipe, models, example }`. */
const assets = [];
let selected = 0;
let latest = null;
let latestRecipe = null;
let timer = 0;
let generation = 0;

const leaf = (path) => path.slice(path.lastIndexOf('/') + 1);
const typeOf = (path) => (path.endsWith('.png') ? 'image/png' : 'application/json');

function say(message, kind = '') {
  status.textContent = message;
  status.className = kind;
}

function kilobytes(bytes) {
  return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`;
}

function download(name, bytes, type) {
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

// ---- Panes (the phone shows one at a time) ---------------------------------

function showPane(name) {
  document.body.dataset.pane = name;
  for (const button of document.querySelectorAll('#panes button')) {
    if (button.dataset.show === name) button.setAttribute('aria-current', 'true');
    else button.removeAttribute('aria-current');
  }
}

for (const button of document.querySelectorAll('#panes button')) {
  button.addEventListener('click', () => showPane(button.dataset.show));
}

// ---- The catalogue ---------------------------------------------------------

/** What an asset is, for the catalogue, read from its recipe text when that parses. */
function describe(asset) {
  try {
    const recipe = JSON.parse(asset.recipe);
    const view = recipe.view ?? 'isometric';
    const kind = recipe.model?.kind === 'file' ? 'model file' : recipe.variants ? `${recipe.variants.length} variants` : 'primitives';
    return { name: recipe.id ?? asset.name, meta: `${view} · ${kind}` };
  } catch {
    return { name: asset.name, meta: 'not valid JSON' };
  }
}

function drawCatalogue() {
  $('assets').replaceChildren(
    ...assets.map((asset, index) => {
      const { name, meta } = describe(asset);
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'asset';
      if (index === selected) button.setAttribute('aria-current', 'true');
      const title = document.createElement('span');
      title.textContent = name;
      const small = document.createElement('small');
      small.textContent = meta;
      button.append(title, small);
      button.addEventListener('click', () => select(index));
      return button;
    }),
  );
}

function select(index, { show = true } = {}) {
  selected = index;
  recipeBox.value = assets[index].recipe;
  if (show) showPane('preview');
  run();
}

function addAsset(asset) {
  assets.push({ models: {}, example: false, ...asset });
  return assets.length - 1;
}

// ---- Model files -----------------------------------------------------------

/** The model files a recipe's text names, whether or not it is valid yet. */
function filesNamedBy(text) {
  try {
    const recipe = JSON.parse(text);
    const models = [recipe.model, ...(recipe.variants ?? []).map((v) => v?.model)];
    return models.filter((m) => m?.kind === 'file' && typeof m.file === 'string').map((m) => m.file);
  } catch {
    return [];
  }
}

/** A starting recipe for a model: isometric, one tile, fitted to it, with a prefab. */
function recipeForModel(fileName) {
  const base = fileName.replace(/\.[^.]+$/, '');
  const id = base.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'model';
  const title = id.split('-').map((word) => word[0].toUpperCase() + word.slice(1)).join(' ');
  return `${JSON.stringify(
    {
      format_version: 1,
      id,
      texture: `textures/${id}.png`,
      tile: { width: 64, height: 32 },
      tile_world: { width: 1.1, height: 0.55 },
      facing: 'south',
      directions: 4,
      footprint: { width: 1, height: 1 },
      prefab: { path: `prefabs/${id}.prefab`, name: title, recipe: `prefabs/${id}.isobake` },
      render: {
        supersample: 4,
        padding: 2,
        palette_snap: true,
        alpha_cutoff: 128,
        outline: { enabled: true, color: '#241d2b' },
        shading: { light: [-0.3, 0.89, 0.35], thresholds: [0.25, 0.5, 0.8] },
      },
      model: { kind: 'file', file: `models/${fileName}`, auto_fit: true, fill: 0.9 },
    },
    null,
    2,
  )}\n`;
}

/** An example's model, fetched from beside its recipe. */
async function fetchModel(file) {
  const response = await fetch(`./examples/${file}`);
  if (!response.ok) throw new Error(`could not load ${file}`);
  return readModelFile(new File([await response.blob()], leaf(file)));
}

/** Read each file the recipe names that the asset does not have yet; returns what is still missing. */
async function supplyModels(asset, recipe) {
  const wanted = recipe.variants.flatMap((variant) => ('file' in variant.model ? [variant.model.file] : []));
  const missing = [];
  for (const file of new Set(wanted)) {
    if (asset.models[file]) continue;
    if (!asset.example) {
      missing.push(file);
      continue;
    }
    say(`Reading ${leaf(file)}…`, 'wait');
    asset.models[file] = await fetchModel(file);
  }
  return missing;
}

// ---- Opening files ---------------------------------------------------------

async function openFiles(files) {
  const list = [...files];
  const recipes = list.filter((file) => ['isobake', 'json'].includes(extensionOf(file.name)));
  const models = list.filter((file) => isModelFile(file.name));
  const rest = list.filter((file) => !recipes.includes(file) && !models.includes(file));
  if (rest.length > 0) {
    say(`${rest.map((file) => file.name).join(', ')}: not a model (.glb, .gltf, .obj) or a recipe (.isobake)`, 'error');
    if (recipes.length + models.length === 0) return;
  }

  let target = selected;
  for (const file of recipes) {
    target = addAsset({ name: file.name.replace(/\.[^.]+$/, ''), recipe: await file.text() });
  }
  let failed = null;
  for (const file of models) {
    let raw;
    try {
      raw = await readModelFile(file);
    } catch (error) {
      failed = error.message;
      continue;
    }
    // A model a recipe is waiting for goes to that recipe; any other becomes its own asset.
    const waiting = filesNamedBy(assets[target].recipe).filter(
      (named) => !assets[target].models[named] && leaf(named).toLowerCase() === file.name.toLowerCase(),
    );
    if (waiting.length > 0) {
      for (const named of waiting) assets[target].models[named] = raw;
    } else {
      target = addAsset({ name: file.name.replace(/\.[^.]+$/, ''), recipe: recipeForModel(file.name), models: { [`models/${file.name}`]: raw } });
    }
  }
  select(target);
  if (failed) say(failed, 'error');
}

$('open').addEventListener('change', async (event) => {
  const files = [...event.target.files];
  event.target.value = '';
  await openFiles(files);
});

let dragDepth = 0;
const carriesFiles = (event) => [...(event.dataTransfer?.types ?? [])].includes('Files');
window.addEventListener('dragenter', (event) => {
  if (!carriesFiles(event)) return;
  dragDepth++;
  $('drop').hidden = false;
});
window.addEventListener('dragover', (event) => {
  if (carriesFiles(event)) event.preventDefault();
});
window.addEventListener('dragleave', (event) => {
  if (!carriesFiles(event)) return;
  dragDepth = Math.max(0, dragDepth - 1);
  if (dragDepth === 0) $('drop').hidden = true;
});
window.addEventListener('drop', (event) => {
  if (!carriesFiles(event)) return;
  event.preventDefault();
  dragDepth = 0;
  $('drop').hidden = true;
  openFiles(event.dataTransfer.files);
});

// ---- Showing a bake --------------------------------------------------------

function drawSheet(image) {
  canvas.width = image.width;
  canvas.height = image.height;
  canvas.getContext('2d').putImageData(new ImageData(image.data, image.width, image.height), 0, 0);
  sizeSheet();
}

function sizeSheet() {
  const factor = Number(zoom.value);
  canvas.style.width = `${canvas.width * factor}px`;
  canvas.style.height = `${canvas.height * factor}px`;
}

/** Every frame on its floor, at the chosen zoom. Cheap enough to redo on a slider. */
function drawCards() {
  if (!latest) return;
  const options = { zoom: Number(zoom.value), grid: grid.checked };
  $('zoom-out').textContent = `${options.zoom}×`;
  $('cards').replaceChildren(
    ...latest.frames.map((frame, index) => {
      const card = document.createElement('figure');
      card.className = 'card';
      const header = document.createElement('header');
      const name = document.createElement('b');
      name.textContent = latest.report.frames[index].direction;
      const size = document.createElement('span');
      size.textContent = `${frame.image.width}×${frame.image.height} · ${frame.footprint.width}×${frame.footprint.height} tiles`;
      header.append(name, size);
      const picture = framePreview(latest, index, options);
      picture.setAttribute('role', 'img');
      picture.setAttribute('aria-label', `${name.textContent} frame on its floor`);
      card.append(header, picture);
      return card;
    }),
  );
  sizeSheet();
}

function row(term, value) {
  const dt = document.createElement('dt');
  dt.textContent = term;
  const dd = document.createElement('dd');
  if (typeof value === 'string') dd.textContent = value;
  else dd.append(value);
  return [dt, dd];
}

function showReport(result) {
  const { report } = result;
  const swatches = document.createElement('div');
  swatches.className = 'swatches';
  for (const color of report.palette) {
    const swatch = document.createElement('i');
    swatch.style.background = color;
    swatch.title = color;
    swatches.append(swatch);
  }
  $('report').replaceChildren(
    ...row('Frame', `${report.canvas.width} × ${report.canvas.height} px`),
    ...row('Anchor', `${report.anchor[0]}, ${report.anchor[1]} (the centre of every frame)`),
    ...row('Sprite scale', `${report.spriteScale.map((v) => v.toFixed(4)).join(' × ')} world units`),
    ...row(`Palette (${report.palette.length})`, swatches),
  );
  $('frames').textContent = `${result.frames.length} frames: ${report.frames.map((f) => f.direction).join(', ')}`;
  $('warnings').replaceChildren(
    ...report.warnings.map((text) => {
      const p = document.createElement('p');
      p.className = 'warn';
      p.textContent = `Warning: ${text}`;
      return p;
    }),
  );
}

function showFiles(files) {
  $('files').replaceChildren(
    ...files.map((file) => {
      const item = document.createElement('li');
      const path = document.createElement('code');
      path.textContent = file.path;
      const size = document.createElement('span');
      size.textContent = kilobytes(file.contents.length);
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = 'Download';
      button.setAttribute('aria-label', `Download ${file.path}`);
      button.addEventListener('click', () => download(leaf(file.path), file.contents, typeOf(file.path)));
      item.append(path, size, button);
      return item;
    }),
  );
}

// ---- Baking ----------------------------------------------------------------

const controls = setupControls({
  recipeBox,
  current: () => ({ recipe: latestRecipe, asset: assets[selected] }),
  rerun: (delay) => schedule(delay),
});
setupTabs();

function fail(message) {
  stage.classList.add('stale');
  $('download-all').disabled = true;
  say(message, 'error');
}

async function run() {
  clearTimeout(timer);
  const mine = ++generation;
  const asset = assets[selected];
  asset.recipe = recipeBox.value;
  drawCatalogue();

  // The controls work on the text, so they need it to be JSON; a recipe the
  // baker rejects for another reason can still be mended with them.
  try {
    JSON.parse(recipeBox.value);
    controls.setEditable(true);
  } catch {
    controls.setEditable(false);
  }

  let recipe;
  let result;
  try {
    recipe = parseRecipe(recipeBox.value, 'recipe');
    const missing = await supplyModels(asset, recipe);
    if (mine !== generation) return;
    if (missing.length > 0) {
      latestRecipe = recipe;
      controls.sync(recipe, asset);
      fail(`This recipe needs the model file ${missing.map(leaf).join(', ')}. Drop it onto the page, or choose it with Open….`);
      return;
    }
    const started = performance.now();
    result = bake(recipe, { models: asset.models });
    result.elapsed = Math.round(performance.now() - started);
  } catch (error) {
    if (mine !== generation) return;
    fail(error instanceof RecipeError || error instanceof Error ? error.message : String(error));
    return;
  }

  latest = result;
  latestRecipe = recipe;
  stage.classList.remove('stale');
  $('download-all').disabled = false;
  drawSheet(result.sheet.image);
  drawCards();
  showReport(result);
  showFiles(result.files);
  controls.sync(recipe, asset);
  say(`Baked ${result.frames.length} frames in ${result.elapsed} ms.`, 'ok');
}

function schedule(delay = 300) {
  clearTimeout(timer);
  timer = setTimeout(run, delay);
}

recipeBox.addEventListener('input', () => schedule());
zoom.addEventListener('input', drawCards);
grid.addEventListener('change', drawCards);
$('save-recipe').addEventListener('click', () => {
  let name = 'recipe';
  try {
    name = JSON.parse(recipeBox.value).id || name;
  } catch {
    // Saving a recipe that does not parse yet is still allowed.
  }
  download(`${name}.isobake`, new TextEncoder().encode(recipeBox.value), 'application/json');
});
$('download-all').addEventListener('click', () => {
  if (!latest) return;
  // Laid out as an asset root, so it unzips straight into a project's assets.
  download(`${latest.report.id}.zip`, zip(latest.files), 'application/zip');
});

// A phone is too narrow for two cards across at the desktop's zoom.
if (window.innerWidth < 700) zoom.value = '2';

try {
  const names = await (await fetch('./examples/index.json')).json();
  const texts = await Promise.all(names.map(async (name) => (await fetch(`./examples/${name}`)).text()));
  names.forEach((name, index) =>
    addAsset({ name: name.replace(/\.isobake$/, ''), recipe: texts[index], example: true }),
  );
  select(0, { show: false });
} catch (error) {
  say(`Could not load the examples: ${error}`, 'error');
}
