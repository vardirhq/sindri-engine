// Same-host standard-material versus legacy control: factors and self-emission.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';
const on = decodePng(readFileSync(process.argv[2]));
const off = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([on.width, on.height, on.channels], [off.width, off.height, off.channels]);
let changed = 0, blueOn = 0, blueOff = 0;
const blue = (data, p) => data[p + 2] > 180 && data[p + 2] > data[p] + 40 && data[p + 2] > data[p + 1] + 20;
for (let p = 0; p < on.data.length; p += on.channels) {
  if ([0, 1, 2].some((c) => Math.abs(on.data[p + c] - off.data[p + c]) > 10)) changed++;
  if (blue(on.data, p)) blueOn++;
  if (blue(off.data, p)) blueOff++;
}
assert(changed > 1000, `authored factors visibly change the scene: ${changed}`);
assert(blueOn > blueOff + 500, `authored emission adds visible blue pixels: ${blueOn}/${blueOff}`);
console.log(`Standard materials: ${changed} changed pixels; ${blueOn - blueOff} added emissive blue pixels`);
