// One material policy changes: the back-facing orange panel becomes visible.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';
const double = decodePng(readFileSync(process.argv[2]));
const single = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([double.width, double.height, double.channels], [single.width, single.height, single.channels]);
let orange = 0, changed = 0;
for (let p = 0; p < double.data.length; p += double.channels) {
  if ([0, 1, 2].some(c => Math.abs(double.data[p + c] - single.data[p + c]) > 10)) changed++;
  if (double.data[p] > single.data[p] + 20 && double.data[p] > double.data[p + 2] + 30) orange++;
}
assert(orange > 500, `double-sided back face adds an orange panel: ${orange}`);
assert(changed > 500, `face visibility survives project delivery: ${changed}`);
console.log(`Material sidedness: ${changed} changed pixels; ${orange} added orange pixels`);
