// Observe script-driven light changes on rendered geometry in one running host.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { decodePng } from './png.mjs';

export function compareLightPhases(pointBytes, spotBytes) {
  const point = decodePng(pointBytes), spot = decodePng(spotBytes);
  assert.deepEqual([point.width, point.height, point.channels], [spot.width, spot.height, spot.channels]);
  const count = (image, channel, other) => {
    let total = 0;
    for (let p = 0; p < image.data.length; p += image.channels) {
      if (image.data[p + channel] > image.data[p + other] + 20) total++;
    }
    return total;
  };
  const red = count(spot, 0, 2) - count(point, 0, 2);
  const blue = count(point, 2, 0) - count(spot, 2, 0);
  assert(red > 100 && blue > 100, `script switches blue point to warm spot: red=${red}, blue=${blue}`);
  console.log(`Live light phases: ${red} added red pixels, ${blue} removed blue pixels`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  compareLightPhases(readFileSync(process.argv[2]), readFileSync(process.argv[3]));
}
