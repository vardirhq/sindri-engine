import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { platformerGoal } from './platformer-goal.mjs';

test('the browser player brakes over the flag and waits for an observed win', async t => {
  const root = await mkdtemp(join(tmpdir(), 'sindri-platformer-player-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, 'assets/content'), { recursive: true });
  await writeFile(join(root, 'assets/sindri.manifest'), JSON.stringify({
    content_root: 'content', entry_scene: 'level.scene',
  }));
  await writeFile(join(root, 'assets/content/level.scene'), JSON.stringify({ entities: [
    { id: 'empty-parent' },
    {
      id: 'level', transform_3d: { position: [0, 0, 0] }, components: {
        'sindri.tilemap': { columns: 8, rows: 1, tiles: Array(8).fill(0), palette: ['grass'] },
      },
    },
    { transform_3d: { position: [5, 0, 0] }, components: { 'sindri.tags': { tags: ['goal'] } } },
  ] }));
  const evidence = {};
  const problems = [];
  const held = new Set();
  let frames = 0;
  const page = {
    keyboard: {
      down: async key => { held.add(key); },
      up: async key => { held.delete(key); },
    },
    waitForTimeout: async () => {
      frames += 1;
      assert.ok(frames <= 3, 'the player must finish when the observer reports a win');
      if (frames === 1) assert.ok(held.has('ArrowRight'), 'run towards the goal');
      if (frames > 1) assert.ok(!held.has('ArrowRight'), 'brake while still airborne over the flag');
      evidence.latest = {
        time: frames / 60, x: 4.6, y: 3, grounded: 0,
        coins: 4, falls: 0, won: frames === 3 ? 1 : 0, jumpHeld: 0,
      };
    },
  };
  await platformerGoal(page, root, evidence, problems);
  assert.deepEqual(problems, []);
  assert.equal(frames, 3, 'being over the flag does not itself count as winning');
  assert.equal(held.size, 0, 'release all inputs after the run');
});
