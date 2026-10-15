// Same-host local lights on/off: only the light carrier's active state changes.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';
const on = decodePng(readFileSync(process.argv[2]));
const off = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([on.width, on.height, on.channels], [off.width, off.height, off.channels]);
let changed = 0, red = 0, blue = 0;
for (let p = 0; p < on.data.length; p += on.channels) {
  if ([0, 1, 2].some((c) => on.data[p + c] > off.data[p + c] + 10)) changed++;
  if (on.data[p] > off.data[p] + 20 && on.data[p] > on.data[p + 2] + 20) red++;
  if (on.data[p + 2] > off.data[p + 2] + 20 && on.data[p + 2] > on.data[p] + 20) blue++;
}
assert(changed > 1000, `point lights brighten visible surfaces: ${changed}`);
assert(red > 100 && blue > 100, `both colored lights contribute: red=${red}, blue=${blue}`);
console.log(`Point lights: ${changed} brightened pixels; ${red} red and ${blue} blue pixels`);
