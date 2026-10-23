// The baker's page: a recipe on the left, the baked sheet on the right.
//
// Everything runs here. `baker/` is the tool's own TypeScript with its types
// stripped (see build.mjs), so a bake in this page is the bake the command line
// does, except for the PNG's compressed bytes: the pixels are identical.
import { bake } from './baker/bake.js';
import { RecipeError, parseRecipe } from './baker/recipe.js';
import { zip } from './baker/zip.js';

const $ = (id) => document.getElementById(id);
const recipeBox = $('recipe');
const status = $('status');
const stage = $('stage');
const canvas = $('sheet');
const zoom = $('zoom');

let latest = null;
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
  applyZoom();
}

function applyZoom() {
  const factor = Number(zoom.value);
  canvas.style.width = `${canvas.width * factor}px`;
  canvas.style.height = `${canvas.height * factor}px`;
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

function run() {
  clearTimeout(timer);
  let result;
  try {
    result = bake(parseRecipe(recipeBox.value, 'recipe'));
  } catch (error) {
    stage.classList.add('stale');
    $('download-all').disabled = true;
    say(error instanceof RecipeError || error instanceof Error ? error.message : String(error), 'error');
    return;
  }
  latest = result;
  stage.classList.remove('stale');
  $('download-all').disabled = false;
  drawSheet(result.sheet.image);
  showReport(result);
  showFiles(result.files);
  say(`Baked ${result.frames.length} frames.`, 'ok');
}

function schedule() {
  clearTimeout(timer);
  timer = setTimeout(run, 300);
}

async function load(name) {
  const response = await fetch(`./examples/${name}`);
  if (!response.ok) throw new Error(`could not load ${name}`);
  recipeBox.value = await response.text();
  run();
}

recipeBox.addEventListener('input', schedule);
zoom.addEventListener('change', applyZoom);
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

try {
  const names = await (await fetch('./examples/index.json')).json();
  const select = $('example');
  for (const name of names) select.append(new Option(name.replace(/\.isobake$/, ''), name));
  await load(names[0]);
} catch (error) {
  say(`Could not load the examples: ${error}`, 'error');
}
