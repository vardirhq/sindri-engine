import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

export function platformerState(evidence, text) {
  const match = text.match(/Platformer state ([\d.]+) ([-\d.]+) ([-\d.]+) ([01]) (\d+) (\d+) ([01]) ([01])$/);
  if (match) {
    const [time, x, y, grounded, coins, falls, won, jumpHeld] = match.slice(1).map(Number);
    evidence.latest = { time, x, y, grounded, coins, falls, won, jumpHeld };
  }
}

// The native goal test's terrain rule, plus braking at the flag for variable
// browser frame timing. Only keyboard events drive gameplay; the fixture's
// observer copies completed state for assertions.
export async function platformerGoal(page, root, evidence, problems) {
  const manifest = JSON.parse(await readFile(join(root, 'assets/sindri.manifest'), 'utf8'));
  const scene = JSON.parse(await readFile(join(root, 'assets', manifest.content_root, manifest.entry_scene), 'utf8'));
  const level = scene.entities.find(entity => entity.id === 'level');
  const tiles = level.components['sindri.tilemap'];
  const corner = level.transform_3d.position;
  const goal = scene.entities.find(entity => entity.components?.['sindri.tags']?.tags.includes('goal'));
  if (!goal) throw new Error('platformer scene has no goal');
  const goalX = goal.transform_3d.position[0];
  const groundAt = column => {
    if (column < 0 || column >= tiles.columns) return null;
    for (let row = 0; row < tiles.rows; row += 1) {
      const tile = tiles.tiles[row * tiles.columns + column];
      if (tile !== null && tiles.palette[tile] !== 'tuft') return corner[1] - row;
    }
    return null;
  };
  let jumpUntil = null;
  let previousTime = -1;
  let jumps = 0;
  let approachingGoal = false;
  const deadline = Date.now() + 90000;
  await page.keyboard.down('ArrowRight');
  try {
    while (Date.now() < deadline) {
      await page.waitForTimeout(1);
      const state = evidence.latest;
      if (!state || state.time === previousTime) continue;
      previousTime = state.time;
      if (state.falls > 0) throw new Error(`platformer fell at x=${state.x}`);
      if (state.won) {
        if (state.coins < 3) throw new Error(`platformer goal reached with only ${state.coins} coins`);
        console.log(`platformer goal: ${state.coins} coins, no falls, ${jumps} jumps, ${state.time}s simulated`);
        return;
      }
      // A jump can carry the hero above the flag sensor. Brake before its
      // centre and let the hero land there rather than running past the goal.
      if (!approachingGoal && state.x >= goalX - 0.5) {
        await page.keyboard.up('ArrowRight');
        approachingGoal = true;
        console.log(`platformer brake: x=${state.x}, y=${state.y}, grounded=${state.grounded}`);
      }
      if (jumpUntil !== null) {
        if (state.time < jumpUntil) continue;
        await page.keyboard.up('Space');
        jumpUntil = null;
        continue;
      }
      // A release must be observed by gameplay before the next press. Sending
      // both in one browser frame leaves the action held and loses its edge.
      if (state.jumpHeld) continue;
      if (approachingGoal) continue;
      const column = Math.floor(state.x - corner[0]);
      const under = groundAt(column);
      const blocked = [1, 2].some(ahead => {
        const top = groundAt(column + ahead);
        return top === null || top > state.y || (under !== null && top < under - 0.5 && ahead === 1);
      });
      if (state.grounded && blocked) {
        await page.keyboard.down('Space');
        jumpUntil = state.time + 40 / 60;
        jumps += 1;
        console.log(`platformer jump ${jumps}: x=${state.x}, coins=${state.coins}, time=${state.time}`);
      }
    }
    throw new Error(`platformer goal timed out: ${JSON.stringify(evidence.latest)}`);
  } catch (error) {
    problems.push(error.message);
  } finally {
    await page.keyboard.up('ArrowRight');
    await page.keyboard.up('Space');
  }
}
