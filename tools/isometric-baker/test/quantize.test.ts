/**
 * Cutting a model's colors down to a few.
 */

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { quantizeColors } from '../src/quantize.ts';

const flat = (...colors: number[][]) => colors.flat();

test('two flat colors are two clusters, each exactly its color', () => {
  const { palette, assignment } = quantizeColors(flat([200, 0, 0], [0, 0, 200], [200, 0, 0], [0, 0, 200], [0, 0, 200]), 2);
  assert.deepEqual(palette, [[0, 0, 200], [200, 0, 0]], 'the more common first');
  assert.deepEqual([...assignment], [1, 0, 1, 0, 0]);
});

test('one cluster is the average', () => {
  const { palette } = quantizeColors(flat([0, 0, 0], [100, 50, 200]), 1);
  assert.deepEqual(palette, [[50, 25, 100]]);
});

test('asking for more clusters than there are colors gives only the colors there are', () => {
  const { palette } = quantizeColors(flat([10, 10, 10], [10, 10, 10], [250, 250, 250]), 8);
  assert.equal(palette.length, 2);
});

test('equal colors are never split apart', () => {
  const { assignment } = quantizeColors(flat(...Array.from({ length: 9 }, () => [90, 40, 20]), [200, 200, 200]), 5);
  assert.equal(new Set([...assignment].slice(0, 9)).size, 1);
});

test('a ramp of grays is cut into bands that cover it in order', () => {
  const grays = Array.from({ length: 64 }, (_, i) => [i * 4, i * 4, i * 4]);
  const { palette, assignment } = quantizeColors(flat(...grays), 4);
  assert.equal(palette.length, 4);
  // Dark inputs land in darker clusters than light ones.
  const cluster = (i: number) => palette[assignment[i]][0];
  assert.ok(cluster(0) < cluster(20) && cluster(20) < cluster(40) && cluster(40) < cluster(63));
});

test('the same colors cut the same way every time', () => {
  let seed = 3;
  const noisy = Array.from({ length: 500 }, () => {
    seed = (seed * 1103515245 + 12345) >>> 0;
    return [seed >>> 24, (seed >>> 16) & 255, (seed >>> 8) & 255];
  });
  const first = quantizeColors(flat(...noisy), 7);
  const second = quantizeColors(flat(...noisy), 7);
  assert.deepEqual(first.palette, second.palette);
  assert.deepEqual([...first.assignment], [...second.assignment]);
  assert.equal(first.palette.length, 7);
});

test('nothing to cut is nothing', () => {
  assert.deepEqual(quantizeColors([], 4).palette, []);
  assert.deepEqual(quantizeColors(flat([1, 2, 3]), 0).palette, []);
});
