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
  const times = (words) => evidence.said.filter((line) => line.includes(words)).length;
  // Waits for the playground to say something again: the hundred balls can
  // set off the domino run on their own, so an earlier line does not count.
  const waitFor = async (words, seconds = 10, before = times(words)) => {
    for (let i = 0; i < seconds * 4 && times(words) <= before; i += 1) await page.waitForTimeout(250);
    if (times(words) <= before) {
      problems.push(`physics playground never said "${words}"; last said: ${evidence.said.slice(-6).join(' | ')}`);
    }
  };
  const press = async (key, words, seconds) => {
    const before = times(words);
    await page.keyboard.press(key);
    await waitFor(words, seconds, before);
  };

  await tapCentre();
  const still = await page.locator('canvas').screenshot();
  await page.keyboard.press('KeyO');
  await page.waitForTimeout(900);
  if (still.equals(await page.locator('canvas').screenshot())) {
    problems.push('physics playground did not move after 100 BALLS');
  }
  await press('KeyV', 'gravity MOON');
  await press('KeyV', 'gravity ZERO-G');
  await press('KeyV', 'gravity UPSIDE DOWN');
  await press('KeyV', 'gravity SIDEWAYS');
  await press('KeyV', 'gravity EARTH');
  const blasts = times('blast');
  await page.keyboard.press('KeyB');
  await tapCentre();
  await waitFor('blast', 10, blasts);
  await press('KeyI', 'debug true');
  // DROP EVERYTHING toggles, and the hundred balls may already have set it
  // off through the domino run, so either answer will do.
  const toggles = () => times('DROP EVERYTHING') + times('joints restored');
  const toggled = toggles();
  await page.keyboard.press('KeyX');
  for (let i = 0; i < 40 && toggles() <= toggled; i += 1) await page.waitForTimeout(250);
  if (toggles() <= toggled) problems.push('physics playground ignored DROP EVERYTHING');
  await press('KeyE', 'toy WRECKING BALL');
  await press('KeyR', 'reset WRECKING BALL');

  // The robot: back round to the test track, walk and kick.
  await press('KeyQ', 'toy THE WHOLE ROOM');
  await press('KeyQ', 'toy DOMINO RUN');
  await press('KeyQ', 'toy TEST TRACK');
  await page.keyboard.down('KeyD');
  await page.waitForTimeout(700);
  await page.keyboard.up('KeyD');
  await press('KeyF', 'robot kick');

  // The domino run, set up again, pushed, and the red button it ends on.
  await press('KeyE', 'toy DOMINO RUN');
  await press('KeyR', 'reset DOMINO RUN');
  await page.waitForTimeout(1200);
  const presses = times('red button');
  await press('Digit1', 'domino pushed');
  await waitFor('red button', 45, presses);

  // The 3D annex: a voxel quarry in a scene of its own, and back again.
  await press('KeyK', 'off to the quarry');
  await page.waitForTimeout(1500);
  await press('Digit2', 'quarry dug under');
  await press('Digit1', 'quarry crates dropped');
  await page.waitForTimeout(1500);
  await press('KeyK', 'quarry back to the room');
}
