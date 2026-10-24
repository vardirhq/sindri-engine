/**
 * Reading the zip a model arrives in.
 */

import assert from 'node:assert/strict';
import { deflateRawSync } from 'node:zlib';
import { test } from 'node:test';

import { crc32 } from '../web/png.ts';
import { unzip } from '../web/unzip.ts';
import { zip } from '../web/zip.ts';

const text = (value: string) => new TextEncoder().encode(value);

/** A zip with deflated entries, as every archiver but ours writes them. */
function deflated(files: { path: string; contents: Uint8Array }[], extra = 0): Uint8Array {
  const locals: Uint8Array[] = [];
  const central: Uint8Array[] = [];
  let offset = 0;
  for (const file of files) {
    const name = text(file.path);
    const packed = file.path.endsWith('/') ? new Uint8Array(0) : deflateRawSync(file.contents);
    const local = new Uint8Array(30 + name.length + extra + packed.length);
    const lv = new DataView(local.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(8, 8, true);
    lv.setUint32(14, crc32(file.contents), true);
    lv.setUint32(18, packed.length, true);
    lv.setUint32(22, file.contents.length, true);
    lv.setUint16(26, name.length, true);
    lv.setUint16(28, extra, true);
    local.set(name, 30);
    local.set(packed, 30 + name.length + extra);
    const entry = new Uint8Array(46 + name.length);
    const cv = new DataView(entry.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(10, 8, true);
    cv.setUint32(16, crc32(file.contents), true);
    cv.setUint32(20, packed.length, true);
    cv.setUint32(24, file.contents.length, true);
    cv.setUint16(28, name.length, true);
    cv.setUint32(42, offset, true);
    entry.set(name, 46);
    locals.push(local);
    central.push(entry);
    offset += local.length;
  }
  const size = central.reduce((total, entry) => total + entry.length, 0);
  const end = new Uint8Array(22);
  const ev = new DataView(end.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(8, files.length, true);
  ev.setUint16(10, files.length, true);
  ev.setUint32(12, size, true);
  ev.setUint32(16, offset, true);
  return Uint8Array.from([...locals, ...central, end].flatMap((part) => [...part]));
}

test('what our own zip writer writes reads back', async () => {
  const files = [
    { path: 'textures/a.png', contents: Uint8Array.from([1, 2, 3, 250]) },
    { path: 'prefabs/b.prefab', contents: text('{"x":1}\n') },
  ];
  assert.deepEqual(await unzip(zip(files)), files);
});

test('deflated entries, folders and an extra field are read the way archivers write them', async () => {
  const big = text('lantern '.repeat(5000));
  const files = [
    { path: 'scene/', contents: new Uint8Array(0) },
    { path: 'scene/scene.gltf', contents: big },
    { path: 'scene/scene.bin', contents: Uint8Array.from({ length: 3000 }, (_, i) => (i * 7) & 255) },
  ];
  const read = await unzip(deflated(files, 5));
  assert.deepEqual(read.map((entry) => entry.path), ['scene/scene.gltf', 'scene/scene.bin'], 'folders are left out');
  assert.deepEqual(read[0].contents, big);
  assert.deepEqual(read[1].contents, files[2].contents);
});

test('something that is not a zip, or is damaged, is a message', async () => {
  await assert.rejects(unzip(text('hello')), /not a zip archive/);
  const good = zip([{ path: 'a.txt', contents: text('hello there') }]);
  const flipped = good.slice();
  flipped[35] ^= 0xff;
  await assert.rejects(unzip(flipped), /checksum|damaged/);
  await assert.rejects(unzip(good.subarray(0, good.length - 30)), /not a zip archive|damaged/);
});

test('an entry that lies about its size is refused', async () => {
  const archive = deflated([{ path: 'a.bin', contents: text('x'.repeat(2000)) }]);
  const lying = archive.slice();
  const view = new DataView(lying.buffer);
  // Shrink the uncompressed size in the central directory entry and the local header.
  view.setUint32(22, 100, true);
  const centralAt = lying.length - 22 - (46 + 5);
  view.setUint32(centralAt + 24, 100, true);
  await assert.rejects(unzip(lying), /larger than the archive says/);
});
