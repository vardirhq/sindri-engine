// A play page holds one game in an iframe, inline or filling the window.
// The game only loads when asked for, so reading about it costs nothing, and
// on a phone (or whenever "Full page" is on) it opens over the whole screen.
const stage = document.querySelector('#stage');
const launch = document.querySelector('#launch');
const pref = document.querySelector('#fullpage-pref');
const expand = document.querySelector('.stage-actions [data-action="fullpage"]');
const KEY = 'sindri.play.fullpage';
const small = matchMedia('(max-width: 700px), (pointer: coarse)');
let frame = null;

function readPref() {
  try {
    const stored = localStorage.getItem(KEY);
    if (stored === '1' || stored === '0') return stored === '1';
  } catch {}
  return small.matches;
}
function savePref(value) {
  try { localStorage.setItem(KEY, value ? '1' : '0'); } catch {}
}

function focusGame() {
  if (!frame) return;
  frame.focus();
  try { frame.contentWindow.focus(); } catch {}
}

function start() {
  if (!frame) {
    frame = document.createElement('iframe');
    frame.title = stage.dataset.title;
    frame.allow = 'fullscreen; gamepad; autoplay; screen-wake-lock';
    frame.allowFullscreen = true;
    frame.addEventListener('load', focusGame);
    frame.src = stage.dataset.src;
    stage.querySelector('.poster').replaceWith(frame);
    stage.classList.add('playing');
    for (const button of document.querySelectorAll('.stage-actions [data-action="restart"]')) button.hidden = false;
  }
  focusGame();
}

function isFull() { return stage.classList.contains('full'); }

function enterFull() {
  if (isFull()) return;
  stage.classList.add('full');
  document.documentElement.classList.add('is-full');
  expand.lastChild.textContent = ' Exit full page';
  // Back (a phone's gesture or button) leaves full page rather than the page.
  history.pushState({ sindriFull: true }, '');
  // On a phone, also ask for real fullscreen to hide the browser's bars. It
  // may be refused (iPhone Safari has none for elements); full page stands
  // on its own without it.
  if (small.matches) {
    const request = stage.requestFullscreen || stage.webkitRequestFullscreen;
    try { request?.call(stage, { navigationUI: 'hide' })?.catch?.(() => {}); } catch {}
  }
  focusGame();
}

function exitFull(fromHistory = false) {
  if (!isFull()) return;
  stage.classList.remove('full');
  document.documentElement.classList.remove('is-full');
  expand.lastChild.textContent = ' Expand';
  const fullscreen = document.fullscreenElement || document.webkitFullscreenElement;
  if (fullscreen) (document.exitFullscreen || document.webkitExitFullscreen).call(document)?.catch?.(() => {});
  if (!fromHistory && history.state?.sindriFull) history.back();
  focusGame();
}

pref.checked = readPref();
pref.addEventListener('change', () => {
  savePref(pref.checked);
  if (!frame) return;
  pref.checked ? enterFull() : exitFull();
});

launch.addEventListener('click', () => {
  start();
  if (pref.checked) enterFull();
});
expand.addEventListener('click', () => {
  if (isFull()) return exitFull();
  start();
  enterFull();
});
for (const button of document.querySelectorAll('[data-action="restart"]')) {
  button.addEventListener('click', () => {
    if (!frame) return;
    frame.src = stage.dataset.src;
    focusGame();
  });
}
stage.querySelector('.stage-controls [data-action="exit"]').addEventListener('click', () => exitFull());
addEventListener('popstate', () => {
  if (isFull() && !history.state?.sindriFull) exitFull(true);
});
// Arriving with #play (from a "Play" link) starts the game straight away;
// browsers still want a tap before sound or real fullscreen.
if (location.hash === '#play') start();
