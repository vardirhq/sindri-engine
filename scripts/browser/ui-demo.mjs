// Drive the Weave Control Room through the exported page: pointer or touch on
// the switches, typing into the callsign, the wheel or a finger on the
// archive, and a row picked. Each is judged by what the Decay script reports,
// not by a painted canvas.
//
// The places are where the demo's own stylesheet lays these out at the two
// viewports CI runs; `project-capture` prints them for any other.
const PLACES = {
  wide: { toggle: [257, 181], checkbox: [257, 237], autopilot: [257, 293], callsign: [257, 349], archive: [703, 276], row: [703, 223] },
  phone: { toggle: [195, 148], checkbox: [195, 200], autopilot: [195, 252], callsign: [195, 304], archive: [195, 584], row: [195, 518] },
};

export async function uiDemo(page, viewport, evidence, problems) {
  const phone = viewport.width < 600;
  const at = PLACES[phone ? 'phone' : 'wide'];
  const settle = () => page.waitForTimeout(400);
  const press = async ([x, y]) => {
    if (phone) await page.touchscreen.tap(x, y);
    else await page.mouse.click(x, y);
    await settle();
  };

  await press(at.toggle);
  await press(at.checkbox);
  await press(at.autopilot); // locked: must report nothing
  await press(at.callsign);
  await page.keyboard.type('Nøva 7');
  await page.keyboard.press('Enter');
  await settle();
  await page.keyboard.press('Escape');

  if (phone) {
    // A finger dragged up the archive, through CDP because Playwright's
    // touchscreen only taps.
    const cdp = await page.context().newCDPSession(page);
    const [x, y] = at.archive;
    const touch = (type, dy) => cdp.send('Input.dispatchTouchEvent', {
      type,
      touchPoints: type === 'touchEnd' ? [] : [{ x, y: y - dy }],
    });
    await touch('touchStart', 0);
    for (let dy = 12; dy <= 144; dy += 12) {
      await touch('touchMove', dy);
      await page.waitForTimeout(30);
    }
    await touch('touchEnd', 144);
  } else {
    await page.mouse.move(...at.archive);
    await page.mouse.wheel(0, 240);
  }
  await settle();
  await page.mouse.move(0, 0);
  // Back to the top, then pick the second row.
  if (!phone) await page.mouse.wheel(0, -2000);
  await settle();
  if (!phone) await press(at.row);

  const expect = (seen, what) => { if (!seen) problems.push(`ui demo: ${what}`); };
  expect(evidence.ready, 'the Decay script never started');
  expect(evidence.toggle, 'the toggle did not change');
  expect(evidence.checkbox, 'the checkbox did not change');
  expect(evidence.typed.includes('Nøva 7'), `typing did not reach the field (${evidence.typed.join(' | ')})`);
  expect(evidence.submitted.includes('Nøva 7'), 'Enter did not submit the callsign');
  expect(evidence.submitted.length === 1, `the callsign was submitted ${evidence.submitted.length} times`);
  expect(evidence.scrolled, 'the archive did not scroll');
  if (!phone) expect(evidence.selected.includes('Arc Imprint'), `the row was not picked (${evidence.selected.join(', ')})`);
  if (phone) expect(evidence.selected.length === 0, 'dragging the archive picked a row');
  expect(!evidence.autopilot, 'the locked switch changed');
}

// Folds one console line into the evidence the checks above read.
export function uiDemoEvidence(evidence, text) {
  if (text.includes('UI demo ready')) evidence.ready = true;
  if (text.includes('UI toggle changed')) evidence.toggle = true;
  if (text.includes('UI checkbox changed')) evidence.checkbox = true;
  if (text.includes('UI autopilot changed')) evidence.autopilot = true;
  const typed = text.match(/UI text changed (.*)$/);
  if (typed) evidence.typed.push(typed[1]);
  const submitted = text.match(/UI submitted (.*)$/);
  if (submitted) evidence.submitted.push(submitted[1]);
  if (text.includes('UI archive scrolled')) evidence.scrolled = true;
  const selected = text.match(/UI selected (.*)$/);
  if (selected) evidence.selected.push(selected[1]);
}

export const freshUiEvidence = () => ({
  ready: false, toggle: false, checkbox: false, autopilot: false,
  typed: [], submitted: [], scrolled: false, selected: [],
});
