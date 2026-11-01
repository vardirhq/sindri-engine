// Captures the mobile prototype in each state the app's design screenshots
// match (`mobile/android/app/src/test/.../DesignScreens.kt`), at the same
// 412 × 915 viewport, by setting the prototype's own state directly.
//
//   node mobile/prototype/capture-states.js [output directory]
//
// Uses the pre-installed Playwright and Chromium. The prototype loads React,
// Babel and its fonts from the network, so this needs network access.
const path = require('path');
const { chromium } = require('playwright');

const file = 'file://' + path.join(__dirname, 'index.html');
const out = path.resolve(process.argv[2] || 'prototype-states');

const STATES = [
  ['01-dashboard', {screen:'dashboard'}],
  ['02-new-template', {screen:'new', newStep:0}],
  ['03-new-name', {screen:'new', newStep:1}],
  ['04-new-creating', {screen:'new', newStep:2, createIdx:2}],
  ['10-editor', {screen:'editor', sheet:null, sel:[], tool:null, mode:'edit'}],
  ['11-more-menu', {moreOpen:true}],
  ['12-add', {moreOpen:false, sheet:'add'}],
  ['13-paint', {sheet:null, tool:'paint'}],
  ['14-assets', {tool:null, sheet:'assets', assetTab:'prefabs'}],
  ['15-import', {sheet:'import'}],
  ['16-slicer', {sheet:'slicer', imported:true}],
  ['17-layers', {sheet:'layers', layerTab:'layers'}],
  ['17b-layers-objects', {sheet:'layers', layerTab:'objects'}],
  ['18-scenes', {sheet:'scenes'}],
  ['19-selected', {sheet:null, sel:['lamp']}],
  ['20-inspector-half', {sheet:'inspector', detent:'half'}],
  ['21-inspector-full', {sheet:'inspector', detent:'full'}],
  ['22-behaviour-blocks', {sheet:'behaviour', behTab:'blocks', detent:'half'}],
  ['23-behaviour-code', {sheet:'behaviour', behTab:'code'}],
  ['24-multi', {sheet:null, sel:['lamp','bridge'], multi:true}],
  ['25-play', {sel:[], multi:false, mode:'play'}],
  ['27-review', {mode:'edit', sheet:'review', runEdits:[{name:'Gloom',label:'Speed',from:2.4,to:1.6,keep:true},{name:'Lamp post',label:'Light radius',from:4.5,to:5.5,keep:false}]}],
  ['26-frozen', {sheet:null, sel:[], mode:'frozen', scrub:0.8}],
  ['26b-frozen-tune', {sel:['lamp'], sheet:'inspector', detent:'half'}],
  ['28-coach', {mode:'edit', sheet:null, sel:[], runEdits:[], coach:'Tap the lamp post next to the gap.'}],
];

(async () => {
  require('fs').mkdirSync(out, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 1, hasTouch: true });
  await page.goto(file);
  await page.waitForTimeout(3500);
  // The prototype's logic object, found through React's fiber on any element.
  const found = await page.evaluate(() => {
    for (const el of document.querySelectorAll('*')) {
      const key = Object.keys(el).find((k) => k.startsWith('__reactFiber'));
      if (!key) continue;
      for (let f = el[key]; f; f = f.return) {
        const s = f.stateNode;
        if (s && s.logic && s.logic.state && 'screen' in s.logic.state) { window.__logic = s.logic; return true; }
      }
    }
    return false;
  });
  if (!found) throw new Error('the prototype did not start');
  for (const [name, patch] of STATES) {
    await page.evaluate((p) => window.__logic.setState(p), patch);
    await page.waitForTimeout(700);
    await page.screenshot({ path: path.join(out, name + '.png') });
  }
  await browser.close();
  console.log('wrote ' + STATES.length + ' states to ' + out);
})();
