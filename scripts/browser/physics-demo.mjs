// Exercise screen controls through the exported Decay project on both viewports.
export async function physicsDemo(page, viewport, evidence, problems) {
  const span = Math.min(1, viewport.width / viewport.height);
  const click = async (x, y) => {
    const px = viewport.width / 2 + x * span * viewport.height / 2;
    const py = (1 - y) * viewport.height / 2;
    if (viewport.width < 600) await page.touchscreen.tap(px, py);
    else await page.mouse.click(px, py);
    await page.waitForTimeout(350);
  };
  await click(-0.6, -0.54); // sensors
  await click(0.6, -0.54); // inside the solid
  for (let i = 0; i < 4; i += 1) await click(0, -0.54); // every mask
  for (const [x, y] of [[-0.6, -0.69], [0, -0.69], [-0.6, -0.84], [0.6, -0.84]]) {
    await click(x, y);
  }
  await click(0, -0.84); // reset
  await click(0.6, -0.69); // fresh falling bodies
  const first = await page.locator('canvas').screenshot();
  await page.waitForTimeout(700);
  if (first.equals(await page.locator('canvas').screenshot())) {
    problems.push('physics bodies did not move after drop');
  }
  // Software WebGPU can run slowly: wait on gameplay events, not wall-clock
  // guesses about how many fixed steps have executed.
  for (let i = 0; i < 15 && (!evidence.sensor || !evidence.contact); i += 1) {
    await page.waitForTimeout(1000);
  }
  if (![...evidence.results].some(result => result.startsWith('Solid / d 3.40'))) {
    problems.push('physics default ray did not hit the solid');
  }
  if (![...evidence.results].some(result => result.startsWith('Sensor / d 1.65'))) {
    problems.push('physics sensor opt-in did not change the closest hit');
  }
  if (!evidence.results.has('Solid / d 0.00 / n 0.0,0.0')) {
    problems.push('physics inside origin did not return the zero-distance contract');
  }
  if (!evidence.results.has('miss')) problems.push('physics mask-none did not miss');
  for (const mask of ['ALL', 'SOLID', 'BODIES', 'NONE']) {
    if (!evidence.masks.has(mask)) problems.push('physics missed mask control ' + mask);
  }
  if (evidence.changes < 11) problems.push('physics demo missed a screen control');
  if (!evidence.sensor || !evidence.contact) problems.push('physics demo missed sensor or collision events');
  await click(0, -0.84);
  await page.waitForTimeout(350);
}
