// Observe script-driven light changes on rendered geometry in one running host.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { decodePng } from './png.mjs';

export function compareLightPhases(pointBytes, spotBytes) {
  const point = decodePng(pointBytes), spot = decodePng(spotBytes);
  assert.deepEqual([point.width, point.height, point.channels], [spot.width, spot.height, spot.channels]);
  // Browser canvas edges can include a CSS border; sample inside the sky.
  const background = (image) => {
    const offset = (Math.min(4, image.height - 1) * image.width + Math.min(4, image.width - 1)) * image.channels;
    return [...image.data.subarray(offset, offset + 3)];
  };
  const count = (image, channel, other) => {
    const sky = background(image);
    let total = 0;
    for (let p = 0; p < image.data.length; p += image.channels) {
      // The clear color also changes; count colored surfaces separately.
      if ([0, 1, 2].every((c) => Math.abs(image.data[p + c] - sky[c]) <= 1)) continue;
      if (image.data[p + channel] > image.data[p + other] + 20) total++;
    }
    return total;
  };
  const red = count(spot, 0, 2) - count(point, 0, 2);
  const blue = count(point, 2, 0) - count(spot, 2, 0);
  assert(red > 100 && blue > 100, `script switches blue point to warm spot: red=${red}, blue=${blue}`);
  const cool = background(point), warm = background(spot);
  assert(cool[2] > cool[0] + 10 && warm[0] > warm[2] + 10,
    `script changes clear color from cool to warm: ${cool} -> ${warm}`);
  console.log(`Live environment backgrounds: ${cool} -> ${warm}`);
  console.log(`Live light phases: ${red} added red pixels, ${blue} removed blue pixels`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  compareLightPhases(readFileSync(process.argv[2]), readFileSync(process.argv[3]));
}
