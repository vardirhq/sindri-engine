/**
 * Models read from a file: where they are put, and what a recipe may say.
 *
 * The file formats are the page's concern. These tests start from triangles,
 * which is the point the baker takes over, and check the conventions every
 * imported model has to meet to stand on a tile: centred on its footprint,
 * sitting on the floor, fitted when asked, and baked like anything else.
 */

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

import { bake } from '../src/bake.ts';
import { type FileModelSpec, type RawModel, meshFromRaw } from '../src/imported.ts';
import { boundsOf } from '../src/model.ts';
import { RecipeError, parseRecipe } from '../src/recipe.ts';
import { barrelParts, boxPart, buildGlb, crateParts } from './glb.ts';

const MODELS = fileURLToPath(new URL('../fixtures/models/', import.meta.url));

const FOOTPRINT = { width: 1, height: 1 };

function spec(overrides: Partial<FileModelSpec> = {}): FileModelSpec {
  return {
    kind: 'file',
    file: 'models/box.glb',
    autoFit: false,
    fill: 1,
    scale: 1,
    rotationY: 0,
    offset: [0, 0, 0],
    materials: {},
    ...overrides,
  };
}

function close(actual: number, expected: number, message?: string) {
  assert.ok(Math.abs(actual - expected) < 1e-9, message ?? `${actual} is not ${expected}`);
}

test('a model is centred on its footprint and stood on the floor, wherever the file put it', () => {
  const raw: RawModel = { parts: [boxPart('Wood', '#b07a42', [2, 1, 1], [5, 3, -7])] };
  const bounds = boundsOf(meshFromRaw(raw, spec(), FOOTPRINT));
  close(bounds.min[1], 0, 'it sits on y = 0');
  close(bounds.max[1], 1);
  close(bounds.min[0], -1);
  close(bounds.max[0], 1);
  close(bounds.min[2], -0.5);
  close(bounds.max[2], 0.5);
});

test('auto fit fills the stated fraction of the footprint along its tighter axis', () => {
  const raw: RawModel = { parts: [boxPart('Wood', '#b07a42', [4, 2, 2], [0, 1, 0])] };
  const bounds = boundsOf(meshFromRaw(raw, spec({ autoFit: true, fill: 0.8 }), FOOTPRINT));
  close(bounds.max[0] - bounds.min[0], 0.8, 'the long side fills 80%');
  close(bounds.max[2] - bounds.min[2], 0.4, 'the other side keeps its proportion');
  close(bounds.max[1] - bounds.min[1], 0.4);

  const wide = boundsOf(meshFromRaw(raw, spec({ autoFit: true, fill: 1 }), { width: 2, height: 1 }));
  close(wide.max[0] - wide.min[0], 2, 'a footprint of two tiles lets it grow');
});

test('scale applies after fitting, and an offset applies last', () => {
  const raw: RawModel = { parts: [boxPart('Wood', '#b07a42', [4, 2, 2], [0, 1, 0])] };
  const mesh = meshFromRaw(raw, spec({ autoFit: true, fill: 0.5, scale: 2, offset: [0.25, 0.5, 0] }), FOOTPRINT);
  const bounds = boundsOf(mesh);
  close(bounds.max[0] - bounds.min[0], 1);
  close((bounds.min[0] + bounds.max[0]) / 2, 0.25, 'nudged along x');
  close(bounds.min[1], 0.5, 'and up');
});

test('turning about Y is measured before fitting, so a quarter turn fills the footprint as seen', () => {
  const raw: RawModel = { parts: [boxPart('Wood', '#b07a42', [4, 1, 1], [0, 0.5, 0])] };
  const bounds = boundsOf(meshFromRaw(raw, spec({ rotationY: 90, autoFit: true }), FOOTPRINT));
  close(bounds.max[2] - bounds.min[2], 1, 'the long side now runs along z');
  close(bounds.max[0] - bounds.min[0], 0.25);
});

test('faces without normals are flat shaded, and normals the file has are kept', () => {
  const part = boxPart('Wood', '#b07a42', [1, 1, 1], [0, 0.5, 0]);
  const bare = meshFromRaw({ parts: [{ ...part, normals: undefined }] }, spec(), FOOTPRINT);
  assert.equal(bare.positions.length / 3, 36, 'unwelded: three vertices for each of twelve triangles');
  const top = [...bare.normals.slice(0, bare.normals.length)].filter((_, i) => i % 3 === 1 && bare.normals[i] > 0.99);
  assert.ok(top.length >= 6, 'the top face points up');

  const kept = meshFromRaw({ parts: [part] }, spec(), FOOTPRINT);
  assert.equal(kept.positions.length / 3, 24, 'a part with normals keeps its own vertices');
});

test('materials are named by the file, shared by name and color, and overridable by name', () => {
  const mesh = meshFromRaw({ parts: crateParts() }, spec(), FOOTPRINT);
  assert.equal(mesh.materials.length, 2, 'two metal parts share one material');
  assert.deepEqual(Object.keys(mesh.ramps), ['Wood', 'Metal']);
  assert.equal(mesh.palette.length, 8, 'each ramp is listed once, four shades each');

  const painted = meshFromRaw({ parts: crateParts() }, spec({ materials: { Wood: { color: '#336699' } } }), FOOTPRINT);
  assert.notDeepEqual(painted.ramps.Wood, mesh.ramps.Wood);
  assert.deepEqual(painted.ramps.Metal, mesh.ramps.Metal);
});

test('a model with nothing to draw is an error that names its file', () => {
  assert.throws(() => meshFromRaw({ parts: [] }, spec(), FOOTPRINT), /models\/box\.glb has no triangles/);
});

const RECIPE = (model: object) =>
  JSON.stringify({
    format_version: 1,
    id: 'crate',
    texture: 'textures/crate.png',
    tile: { width: 64, height: 32 },
    directions: 4,
    footprint: { width: 1, height: 1 },
    model,
  });

test('a recipe naming a model file bakes once the file is supplied', () => {
  const recipe = parseRecipe(RECIPE({ kind: 'file', file: 'models/crate.glb', auto_fit: true, fill: 0.9 }), 'crate');
  const models = { 'models/crate.glb': { parts: crateParts() } };

  const first = bake(recipe, { models });
  assert.equal(first.frames.length, 4);
  assert.equal(first.report.warnings.length, 0, first.report.warnings.join('; '));
  assert.ok(first.frames.every((frame) => frame.content), 'every direction drew something');

  const second = bake(recipe, { models });
  assert.ok(Buffer.from(first.files[0].contents).equals(second.files[0].contents), 'bakes the same bytes twice');

  const turned = bake(
    parseRecipe(RECIPE({ kind: 'file', file: 'models/crate.glb', auto_fit: true, fill: 0.9, rotation_y: 45 }), 'crate'),
    { models },
  );
  assert.ok(!Buffer.from(first.files[0].contents).equals(turned.files[0].contents), 'turning it changes the picture');
});

test('a smooth model bakes with the same four shades per material as anything else', () => {
  const recipe = parseRecipe(RECIPE({ kind: 'file', file: 'models/barrel.glb', auto_fit: true, fill: 0.85 }), 'barrel');
  const result = bake(recipe, { models: { 'models/barrel.glb': { parts: barrelParts() } } });
  assert.equal(result.report.warnings.length, 0, result.report.warnings.join('; '));
  assert.ok(result.report.palette.length <= 9, `${result.report.palette.length} colors`);
});

test('baking without the file says what is missing and who reads files', () => {
  const recipe = parseRecipe(RECIPE({ kind: 'file', file: 'models/crate.glb' }), 'crate');
  assert.throws(() => bake(recipe), /model file "models\/crate\.glb" was not provided/);
});

test('a model that overhangs its footprint is refused like any other', () => {
  const recipe = parseRecipe(RECIPE({ kind: 'file', file: 'models/crate.glb', scale: 2 }), 'crate');
  assert.throws(() => bake(recipe, { models: { 'models/crate.glb': { parts: crateParts() } } }), /footprint claims 1/);
});

test('a recipe may not name a path outside the project or a format we do not read', () => {
  const refuse = (model: object, pattern: RegExp) =>
    assert.throws(() => parseRecipe(RECIPE(model), 'crate'), (error) => error instanceof RecipeError && pattern.test(error.message));

  refuse({ kind: 'file', file: '../chair.glb' }, /relative path inside the project/);
  refuse({ kind: 'file', file: '/etc/chair.glb' }, /relative path inside the project/);
  refuse({ kind: 'file', file: 'C:\\models\\chair.glb' }, /relative path inside the project/);
  refuse({ kind: 'file', file: 'models/chair.fbx' }, /\.glb, \.gltf, \.obj/);
  refuse({ kind: 'file' }, /missing "file"/);
  refuse({ kind: 'file', file: 'a.glb', fill: 0 }, /fill/);
  refuse({ kind: 'file', file: 'a.glb', fill: 1.5 }, /fill/);
  refuse({ kind: 'file', file: 'a.glb', scale: -1 }, /scale/);
  refuse({ kind: 'file', file: 'a.glb', parts: [] }, /unknown field "parts"/);
  refuse({ kind: 'mesh' }, /expected "primitives" or "file"/);
});

test('the example model files are still what the generator makes', async () => {
  assert.ok(Buffer.from(buildGlb(crateParts())).equals(await readFile(`${MODELS}crate.glb`)), 'crate.glb is stale; run node fixtures/models/make.ts');
  assert.ok(Buffer.from(buildGlb(barrelParts())).equals(await readFile(`${MODELS}barrel.glb`)), 'barrel.glb is stale; run node fixtures/models/make.ts');
});

test('the example recipes parse and name the files beside them', async () => {
  for (const name of ['crate', 'barrel']) {
    const text = await readFile(`${MODELS}${name}.isobake`, 'utf8');
    const recipe = parseRecipe(text, name);
    assert.equal((recipe.variants[0].model as FileModelSpec).file, `models/${name}.glb`);
  }
});
