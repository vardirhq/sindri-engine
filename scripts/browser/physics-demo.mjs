// Plays the Physics Playground in a real browser: the keyboard's global
// controls, a pointer tool on the canvas, and the room visibly moving.
export async function physicsDemo(page, viewport, evidence, problems) {
  const touch = viewport.width < 600;
  const tapCentre = async () => {
    const x = viewport.width / 2;
    const y = viewport.height * 0.55;
    if (touch) await page.touchscreen.tap(x, y);
    else await page.mouse.click(x, y);
    await page.waitForTimeout(300);
  };
  const said = (words) => evidence.said.some((line) => line.includes(words));
  const waitFor = async (words, seconds = 10) => {
    for (let i = 0; i < seconds * 4 && !said(words); i += 1) await page.waitForTimeout(250);
    if (!said(words)) problems.push(`physics playground never said "${words}"`);
  };

  await tapCentre();
  const still = await page.locator('canvas').screenshot();
  await page.keyboard.press('KeyO');
  await page.waitForTimeout(900);
  if (still.equals(await page.locator('canvas').screenshot())) {
    problems.push('physics playground did not move after 100 BALLS');
  }
  await page.keyboard.press('KeyV');
  await waitFor('gravity MOON');
  await page.keyboard.press('KeyB');
  await tapCentre();
  await waitFor('blast');
  await page.keyboard.press('KeyX');
  await waitFor('DROP EVERYTHING');
  await page.keyboard.press('KeyE');
  await waitFor('toy WRECKING BALL');
  await page.keyboard.press('KeyR');
  await waitFor('reset WRECKING BALL');
}
