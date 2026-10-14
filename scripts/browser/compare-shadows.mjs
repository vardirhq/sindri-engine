// Compare fixed-workload captures made by the same host with shadows on/off.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodePng } from './png.mjs';

const on = decodePng(readFileSync(process.argv[2]));
const off = decodePng(readFileSync(process.argv[3]));
assert.deepEqual([on.width, on.height, on.channels], [off.width, off.height, off.channels]);
let changed = 0, darkened = 0, receiverDarkened = 0;
for (let pixel = 0; pixel < on.data.length; pixel += on.channels) {
  let difference = 0, darkness = 0;
  for (let channel = 0; channel < 3; channel++) {
    const delta = off.data[pixel + channel] - on.data[pixel + channel];
    difference = Math.max(difference, Math.abs(delta));
    darkness += delta;
  }
  if (difference > 3) changed++;
  if (darkness > 9) darkened++;
  // Only the imported ground is green in the dedicated receiver control.
  if (off.data[pixel + 1] > off.data[pixel] + 20
      && off.data[pixel + 1] > off.data[pixel + 2] + 10
      && off.data[pixel + 1] > on.data[pixel + 1] + 10) receiverDarkened++;
}
assert(changed > 100, `authored shadows change visible pixels: ${changed}`);
assert(darkened > 100, `authored shadows darken visible pixels: ${darkened}`);
console.log(`Authored shadows: ${changed} changed pixels, ${darkened} darkened pixels`);

if (process.argv.includes('--green-receiver')) {
  assert(receiverDarkened > 100, `imported green receiver darkens: ${receiverDarkened}`);
  console.log(`Imported model shadow: ${receiverDarkened} green receiver pixels darkened`);
}
