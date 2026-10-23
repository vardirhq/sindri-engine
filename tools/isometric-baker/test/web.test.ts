/**
 * The page the baker ships.
 *
 * It is built from the same sources the command line runs, with two files
 * swapped for browser versions, so the claims worth testing are that the build
 * runs, that a bake through it is the bake the tool does, and that the two
 * browser-only writers produce files other software reads.
 */

import assert from 'node:assert/strict';
import { mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { test } from 'node:test';

import { bake } from '../src/bake.ts';
import { firstDifference } from '../src/image.ts';
import { decodePng } from '../src/png.ts';
import { parseRecipe } from '../src/recipe.ts';
import { encodePng } from '../web/png.ts';
import { zip } from '../web/zip.ts';
import { EXAMPLES, build } from '../web/build.mjs';

const TOOL = fileURLToPath(new URL('../', import.meta.url));

test('a bake through the built page is the bake the tool does', async () => {
  const out = await build(await mkdtemp(join(tmpdir(), 'baker-web-')));
  const web = await import(pathToFileURL(join(out, 'baker/bake.js')).href);
  const webRecipe = await import(pathToFileURL(join(out, 'baker/recipe.js')).href);

  for (const [path] of EXAMPLES) {
    const json = await readFile(join(TOOL, path), 'utf8');
    const native = bake(parseRecipe(json, path));
    const browser = web.bake(webRecipe.parseRecipe(json, path));

    assert.deepEqual(
      browser.files.map((file: { path: string }) => file.path),
      native.files.map((file) => file.path),
    );
    for (const [index, file] of native.files.entries()) {
      const other = browser.files[index].contents;
      if (file.path.endsWith('.png')) {
        const at = firstDifference(decodePng(Buffer.from(other)), decodePng(Buffer.from(file.contents)));
        assert.equal(at, null, `${path}: ${file.path} differs in the browser at ${at?.x},${at?.y}`);
      } else {
        assert.ok(Buffer.from(other).equals(file.contents), `${path}: ${file.path} differs in the browser`);
      }
    }
  }
});

test('the page offers every example it ships', async () => {
  const out = await build(await mkdtemp(join(tmpdir(), 'baker-web-')));
  const listed = JSON.parse(await readFile(join(out, 'examples/index.json'), 'utf8')) as string[];
  assert.equal(listed.length, EXAMPLES.length);
  for (const name of listed) assert.ok((await readFile(join(out, 'examples', name), 'utf8')).includes('format_version'));
});

test('the browser PNG writer round-trips pixels and compresses a sheet', () => {
  const width = 97;
  const height = 61;
  const data = new Uint8ClampedArray(width * height * 4);
  // Flat runs, a gradient and noise: every kind of thing a deflater meets.
  let seed = 7;
  for (let i = 0; i < data.length; i += 4) {
    const pixel = i / 4;
    seed = (seed * 1103515245 + 12345) >>> 0;
    const noisy = pixel % 53 < 9;
    data[i] = noisy ? seed >>> 24 : pixel % width;
    data[i + 1] = noisy ? seed >>> 16 : 40;
    data[i + 2] = 200;
    data[i + 3] = pixel % 7 < 3 ? 0 : 255;
  }
  const image = { data, width, height };
  const png = encodePng(image);

  assert.equal(firstDifference(decodePng(Buffer.from(png)), image), null);
  assert.ok(png.length < data.length / 2, `${png.length} bytes for ${data.length} of pixels`);
  assert.deepEqual(png, encodePng(image), 'the same pixels are the same bytes');
});

test('a zip lists its entries and stores them intact', () => {
  const entries = [
    { path: 'textures/a.png', contents: Uint8Array.from([1, 2, 3, 250]) },
    { path: 'prefabs/b.prefab', contents: new TextEncoder().encode('{"x":1}\n') },
  ];
  const bytes = zip(entries);
  const view = new DataView(bytes.buffer);

  const end = bytes.length - 22;
  assert.equal(view.getUint32(end, true), 0x06054b50);
  assert.equal(view.getUint16(end + 10, true), entries.length);

  let at = view.getUint32(end + 16, true);
  for (const entry of entries) {
    assert.equal(view.getUint32(at, true), 0x02014b50);
    const size = view.getUint32(at + 20, true);
    const nameLength = view.getUint16(at + 28, true);
    const local = view.getUint32(at + 42, true);
    const name = new TextDecoder().decode(bytes.subarray(at + 46, at + 46 + nameLength));
    assert.equal(name, entry.path);
    const start = local + 30 + nameLength;
    assert.deepEqual(bytes.subarray(start, start + size), entry.contents);
    at += 46 + nameLength;
  }
});
