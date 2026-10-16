// Same geometry, transforms, lights and material; only authored normals differ.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';
const smooth = decodePng(readFileSync(process.argv[2]));
const flat = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([smooth.width, smooth.height, smooth.channels], [flat.width, flat.height, flat.channels]);
let changed = 0, maximum = 0;
for (let p = 0; p < smooth.data.length; p += smooth.channels) {
  const difference = Math.max(...[0, 1, 2].map((c) => Math.abs(smooth.data[p + c] - flat.data[p + c])));
  if (difference > 10) changed++;
  maximum = Math.max(maximum, difference);
}
assert(changed > 1000, `authored normals visibly change surface shading: ${changed}`);
assert(maximum > 30, `normal differences survive capture: ${maximum}`);
console.log(`Surface normals: ${changed} changed pixels; maximum channel difference ${maximum}`);
