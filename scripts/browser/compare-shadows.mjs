// Compare fixed-workload captures made by the same host with shadows on/off.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';

const on = decodePng(readFileSync(process.argv[2]));
const off = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([on.width, on.height, on.channels], [off.width, off.height, off.channels]);
let changed = 0, darkened = 0;
for (let pixel = 0; pixel < on.data.length; pixel += on.channels) {
  let difference = 0, darkness = 0;
  for (let channel = 0; channel < 3; channel++) {
    const delta = off.data[pixel + channel] - on.data[pixel + channel];
    difference = Math.max(difference, Math.abs(delta));
    darkness += delta;
  }
  if (difference > 3) changed++;
  if (darkness > 9) darkened++;
}
assert(changed > 100, `authored shadows change visible pixels: ${changed}`);
assert(darkened > 100, `authored shadows darken visible pixels: ${darkened}`);
console.log(`Authored shadows: ${changed} changed pixels, ${darkened} darkened pixels`);
