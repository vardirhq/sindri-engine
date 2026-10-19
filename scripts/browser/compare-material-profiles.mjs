// One shared asset edit changes both a cube and an imported model.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';
const original = decodePng(readFileSync(process.argv[2]));
const edited = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([original.width, original.height, original.channels], [edited.width, edited.height, edited.channels]);
let green = 0, left = 0, right = 0;
for (let p = 0; p < edited.data.length; p += edited.channels) {
  if (edited.data[p + 1] > original.data[p + 1] + 25 && edited.data[p + 1] > edited.data[p] + 25) {
    green++;
    const x = (p / edited.channels) % edited.width;
    if (x < edited.width / 2) left++; else right++;
  }
}
assert(green > 500, `shared material edit adds green emission: ${green}`);
assert(left > 100 && right > 100, `both shared surfaces change: ${left}/${right}`);
console.log(`Shared material profile: ${green} added green pixels (${left} left, ${right} right)`);
