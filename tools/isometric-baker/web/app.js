// The baker's page: a recipe on the left, each baked frame in context in the
// middle, and the render settings on the right.
//
// Everything runs here. `baker/` is the tool's own TypeScript with its types
// stripped (see build.mjs), so a bake in this page is the bake the command line
// does, except for the PNG's compressed bytes: the pixels are identical.
//
// The recipe text is the one source of truth. The sliders read from the recipe
// the last bake parsed, and move a slider by rewriting that text and baking it
// again, so what is on the left is always what Save recipe will write.
import { bake } from './baker/bake.js';
import { RecipeError, parseRecipe } from './baker/recipe.js';
import { zip } from './baker/zip.js';
import { framePreview } from './context.js';

const $ = (id) => document.getElementById(id);
const recipeBox = $('recipe');
const status = $('status');
const stage = $('stage');
const canvas = $('sheet');
const zoom = $('zoom');
const grid = $('grid');
const controls = $('controls');

let latest = null;
let latestRecipe = null;
let timer = 0;

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

const typeOf = (path) => (path.endsWith('.png') ? 'image/png' : 'application/json');
const leaf = (path) => path.slice(path.lastIndexOf('/') + 1);

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
      card.style.margin = '0';
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

// ---- The render settings ---------------------------------------------------

const DEGREES = 180 / Math.PI;

/** Where the light is, as the two angles a person thinks in. */
function lightAngles([x, y, z]) {
  const length = Math.hypot(x, y, z) || 1;
  return {
    azimuth: Math.round(((Math.atan2(z, x) * DEGREES) + 360) % 360),
    elevation: Math.round(Math.asin(y / length) * DEGREES),
  };
}

function lightFrom(azimuth, elevation) {
  const flat = Math.cos(elevation / DEGREES);
  const round = (v) => Math.round(v * 1000) / 1000;
  return [
    round(flat * Math.cos(azimuth / DEGREES)),
    round(Math.sin(elevation / DEGREES)),
    round(flat * Math.sin(azimuth / DEGREES)),
  ];
}

/** What each control shows, read from a recipe the baker has parsed. */
const READ = {
  supersample: (r) => r.render.supersample,
  alpha_cutoff: (r) => r.render.alphaCutoff,
  padding: (r) => r.render.padding,
  palette_snap: (r) => r.render.paletteSnap,
  outline_enabled: (r) => r.render.outline.enabled,
  outline_color: (r) => r.render.outline.color,
  azimuth: (r) => lightAngles(r.render.shading.light).azimuth,
  elevation: (r) => lightAngles(r.render.shading.light).elevation,
  band0: (r) => r.render.shading.thresholds[0],
  band1: (r) => r.render.shading.thresholds[1],
  band2: (r) => r.render.shading.thresholds[2],
};

const LABEL = {
  supersample: (v) => `${v}×`,
  alpha_cutoff: (v) => `${v}`,
  padding: (v) => `${v}px`,
  azimuth: (v) => `${v}°`,
  elevation: (v) => `${v}°`,
  band0: (v) => v.toFixed(2),
  band1: (v) => v.toFixed(2),
  band2: (v) => v.toFixed(2),
};

const inputs = Object.fromEntries(
  [...controls.querySelectorAll('[data-setting]')].map((input) => [input.dataset.setting, input]),
);

/** Move the controls to match the recipe, leaving alone the one being dragged. */
function syncControls(recipe) {
  for (const [key, input] of Object.entries(inputs)) {
    if (input === document.activeElement) continue;
    const value = READ[key](recipe);
    if (input.type === 'checkbox') input.checked = value;
    else if (input.type === 'color') input.value = /^#[0-9a-f]{6}$/i.test(value) ? value : '#000000';
    else input.value = value;
    const out = controls.querySelector(`[data-out="${key}"]`);
    if (out) out.textContent = LABEL[key](Number(value));
  }
}

/** Write one setting into the recipe's text, then bake it again. */
function change(key, value) {
  let recipe;
  try {
    recipe = JSON.parse(recipeBox.value);
  } catch {
    return;
  }
  const render = (recipe.render ??= {});
  // A shading block needs both its keys, so start it from what the bake is using.
  const shading = () =>
    (render.shading ??= {
      light: [...latestRecipe.render.shading.light],
      thresholds: [...latestRecipe.render.shading.thresholds],
    });

  if (key === 'supersample' || key === 'alpha_cutoff' || key === 'padding' || key === 'palette_snap') {
    render[key] = value;
  } else if (key === 'outline_enabled') {
    (render.outline ??= {}).enabled = value;
  } else if (key === 'outline_color') {
    (render.outline ??= {}).color = value;
  } else if (key === 'azimuth' || key === 'elevation') {
    const now = lightAngles(shading().light);
    const next = { ...now, [key]: value };
    shading().light = lightFrom(next.azimuth, next.elevation);
  } else {
    // A band edge may not pass its neighbours: the baker requires them ascending.
    const index = Number(key.slice(4));
    const edges = shading().thresholds;
    edges[index] = Math.min(Math.max(value, edges[index - 1] ?? 0), edges[index + 1] ?? 1);
  }
  recipeBox.value = `${JSON.stringify(recipe, null, 2)}\n`;
  schedule(0);
}

for (const [key, input] of Object.entries(inputs)) {
  input.addEventListener('input', () => {
    const value = input.type === 'checkbox' ? input.checked : input.type === 'color' ? input.value : Number(input.value);
    const out = controls.querySelector(`[data-out="${key}"]`);
    if (out && LABEL[key]) out.textContent = LABEL[key](value);
    change(key, value);
  });
}

// ---- Baking ----------------------------------------------------------------

function run() {
  clearTimeout(timer);
  // Sliders work on the text, so they need it to be JSON; a recipe the baker
  // rejects for another reason can still be mended with them.
  try {
    JSON.parse(recipeBox.value);
    controls.disabled = false;
  } catch {
    controls.disabled = true;
  }

  let result;
  let recipe;
  const started = performance.now();
  try {
    recipe = parseRecipe(recipeBox.value, 'recipe');
    result = bake(recipe);
  } catch (error) {
    stage.classList.add('stale');
    $('download-all').disabled = true;
    say(error instanceof RecipeError || error instanceof Error ? error.message : String(error), 'error');
    return;
  }
  const elapsed = Math.round(performance.now() - started);
  latest = result;
  latestRecipe = recipe;
  stage.classList.remove('stale');
  $('download-all').disabled = false;
  drawSheet(result.sheet.image);
  drawCards();
  showReport(result);
  showFiles(result.files);
  syncControls(recipe);
  say(`Baked ${result.frames.length} frames in ${elapsed} ms.`, 'ok');
}

function schedule(delay = 300) {
  clearTimeout(timer);
  timer = setTimeout(run, delay);
}

async function load(name) {
  const response = await fetch(`./examples/${name}`);
  if (!response.ok) throw new Error(`could not load ${name}`);
  recipeBox.value = await response.text();
  run();
}

recipeBox.addEventListener('input', () => schedule());
zoom.addEventListener('input', drawCards);
grid.addEventListener('change', drawCards);
$('example').addEventListener('change', (event) => load(event.target.value).catch((e) => say(String(e), 'error')));
$('open').addEventListener('change', async (event) => {
  const [file] = event.target.files;
  if (!file) return;
  recipeBox.value = await file.text();
  event.target.value = '';
  run();
});
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
  const select = $('example');
  for (const name of names) select.append(new Option(name.replace(/\.isobake$/, ''), name));
  await load(names[0]);
} catch (error) {
  say(`Could not load the examples: ${error}`, 'error');
}
