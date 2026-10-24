/**
 * The page's file reader, run under Node against real files.
 *
 * Three.js is the reader and the page's only dependency, so these run when it is
 * installed (`npm ci --prefix web`, which CI does) and say they were skipped
 * when it is not. They check that what a loader makes of a file is the triangles
 * and colors the baker expects: the same colors the generator wrote, in sRGB,
 * and the same geometry.
 */

import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

import { bake } from '../src/bake.ts';
import { type RawModel } from '../src/imported.ts';
import { parseRecipe } from '../src/recipe.ts';
import { barrelParts, crateParts } from './glb.ts';

const MODELS = fileURLToPath(new URL('../fixtures/models/', import.meta.url));

type Reader = (file: File) => Promise<RawModel>;

async function reader(): Promise<Reader | null> {
  try {
    await access(fileURLToPath(new URL('../web/node_modules/three/package.json', import.meta.url)));
  } catch {
    return null;
  }
  // @ts-expect-error: the page's reader is plain JavaScript.
  const models = (await import('../web/models.js')) as { readModelFile: Reader };
  return models.readModelFile;
}

async function load(read: Reader, name: string): Promise<RawModel> {
  return read(new File([await readFile(`${MODELS}${name}`)], name));
}

test('a .glb is read into the triangles and sRGB colors it was written with', async (t) => {
  const read = await reader();
  if (!read) return t.skip('three is not installed: npm ci --prefix web');

  for (const [file, parts] of [['crate.glb', crateParts()], ['barrel.glb', barrelParts()]] as const) {
    const raw = await load(read, file);
    assert.deepEqual(
      raw.parts.map((part) => [part.name, part.color]),
      parts.map((part) => [part.name, part.color]),
      `${file}: names and colors`,
    );
    raw.parts.forEach((part, index) => {
      assert.equal(part.indices.length, parts[index].indices.length, `${file}: triangles of part ${index}`);
      assert.ok(part.normals, `${file}: the file's own normals are kept`);
    });
  }
});

test('a model read from a file bakes, and bakes the same twice', async (t) => {
  const read = await reader();
  if (!read) return t.skip('three is not installed: npm ci --prefix web');

  const recipe = parseRecipe(await readFile(`${MODELS}barrel.isobake`, 'utf8'), 'barrel');
  const models = { 'models/barrel.glb': await load(read, 'barrel.glb') };
  const first = bake(recipe, { models });
  assert.equal(first.report.warnings.length, 0, first.report.warnings.join('; '));
  assert.ok(first.frames.every((frame) => frame.content));
  assert.ok(Buffer.from(first.files[0].contents).equals(bake(recipe, { models }).files[0].contents));
});

test('an .obj with no materials is read, flat shaded, with names to override by', async (t) => {
  const read = await reader();
  if (!read) return t.skip('three is not installed: npm ci --prefix web');

  const obj = 'v 0 0 0\nv 1 0 0\nv 0 0 1\nv 0 1 0\nf 1 2 3\nf 1 3 4\n';
  const raw = await read(new File([obj], 'wedge.obj'));
  assert.equal(raw.parts.length, 1);
  assert.match(raw.parts[0].name, /^Material \d+$/);
  assert.equal(raw.parts[0].indices.length, 6);
});

test('a file that is not a model says which file and why', async (t) => {
  const read = await reader();
  if (!read) return t.skip('three is not installed: npm ci --prefix web');

  await assert.rejects(read(new File(['not a model'], 'broken.glb')), /broken\.glb could not be read/);
  await assert.rejects(read(new File(['# nothing\n'], 'empty.obj')), /empty\.obj has no triangles/);
});
