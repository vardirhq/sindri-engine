// Compare shared-host captures while isolating clock, shadow and local-light edits.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { decodePng } from './png.mjs';

const decode = (bytes) => decodePng(bytes);
function sameSize(images) {
  for (const image of images) assert.deepEqual([image.width, image.height, image.channels],
    [images[0].width, images[0].height, images[0].channels]);
}
function sky(image) {
  const p = (4 * image.width + 4) * image.channels;
  return [...image.data.subarray(p, p + 3)];
}
export function compareCycle(bytes) {
  const images = bytes.map(decode);
  sameSize(images);
  const backgrounds = images.map(sky);
  const midday = backgrounds[1], night = backgrounds[3];
  assert(midday[2] > midday[0] + 20, `blue midday sky: ${midday}`);
  assert(midday.reduce((a,b) => a+b) > night.reduce((a,b) => a+b) + 100, 'night sky is darker');
  const colors = new Set(backgrounds.map(color => color.join(',')));
  assert(backgrounds[2][0] > backgrounds[2][2] + 10, `warm sunset sky: ${backgrounds[2]}`);
  assert(colors.size === 4, `clock changes sunrise/midday/sunset/night backgrounds: ${backgrounds}`);
  console.log(`Day/night sky phases: ${backgrounds.map(c => c.join(',')).join(' -> ')}`);
}
export function compareNight(onBytes, offBytes) {
  const [on, off] = [onBytes, offBytes].map(decode);
  sameSize([on, off]);
  assert.deepEqual(sky(on), sky(off), 'night controls keep the same environment');
  let brightened = 0;
  for (let p = 0; p < on.data.length; p += on.channels) {
    if ([0,1,2].some(c => on.data[p+c] > off.data[p+c] + 10)) brightened++;
  }
  assert(brightened > 1000, `mounted lights illuminate imported geometry with the sun off: ${brightened}`);
  console.log(`Mounted night lights: ${brightened} brightened pixels`);
}
function shadowMask(on, off) {
  const pixels = new Set();
  let x = 0;
  for (let p = 0; p < on.data.length; p += on.channels) {
    if (off.data[p+1] > off.data[p] + 20 && off.data[p+1] > off.data[p+2] + 10
        && off.data[p+1] > on.data[p+1] + 10) {
      const index = p / on.channels;
      pixels.add(index);
      x += index % on.width;
    }
  }
  assert(pixels.size > 100, `imported green ground receives shadows: ${pixels.size}`);
  return { pixels, x: x / pixels.size };
}
export function compareShadowDirections(bytes) {
  const images = bytes.map(decode);
  sameSize(images);
  const morning = shadowMask(images[0], images[1]), afternoon = shadowMask(images[2], images[3]);
  const removed = [...morning.pixels].filter(p => !afternoon.pixels.has(p)).length;
  const added = [...afternoon.pixels].filter(p => !morning.pixels.has(p)).length;
  assert(removed > 100 && added > 100 && Math.abs(morning.x - afternoon.x) > 3,
    `stationary imported casters follow the scripted sun: removed=${removed}, added=${added}, x=${morning.x},${afternoon.x}`);
  console.log(`Sun shadow direction: ${removed} removed and ${added} added receiver pixels; centroid x ${morning.x.toFixed(2)} -> ${afternoon.x.toFixed(2)}`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const bytes = process.argv.slice(3).map(path => readFileSync(path));
  if (process.argv[2] === 'cycle') compareCycle(bytes);
  else if (process.argv[2] === 'shadows') compareShadowDirections(bytes);
  else if (process.argv[2] === 'night') compareNight(...bytes);
  else throw new Error('expected cycle, shadows or night');
}
