const controls = document.querySelector('.directory-controls');
const search = document.querySelector('#directory-search');
const buttons = [...document.querySelectorAll('[data-category]')];
const entries = [...document.querySelectorAll('.directory-entry')];
const status = document.querySelector('#directory-status');
const empty = document.querySelector('#directory-empty');
let category = 'all';

function filter(updateUrl = true) {
  const terms = search.value.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  let count = 0;
  for (const entry of entries) {
    const matches = (category === 'all' || entry.dataset.category === category)
      && terms.every(term => entry.dataset.search.includes(term));
    entry.hidden = !matches;
    if (matches) count++;
  }
  for (const button of buttons) {
    button.setAttribute('aria-pressed', String(button.dataset.category === category));
  }
  status.textContent = count + (count === 1 ? ' entry' : ' entries');
  empty.hidden = count > 0;
  if (updateUrl) {
    const url = new URL(location.href);
    search.value.trim() ? url.searchParams.set('q', search.value.trim()) : url.searchParams.delete('q');
    category === 'all' ? url.searchParams.delete('type') : url.searchParams.set('type', category);
    history.replaceState(null, '', url);
  }
}
function readUrl() {
  const params = new URLSearchParams(location.search);
  search.value = params.get('q') || '';
  category = buttons.some(button => button.dataset.category === params.get('type'))
    ? params.get('type') : 'all';
  filter(false);
}
controls.hidden = false;
controls.querySelector('form').addEventListener('submit', event => event.preventDefault());
search.addEventListener('input', () => filter());
for (const button of buttons) {
  button.addEventListener('click', () => { category = button.dataset.category; filter(); });
}
document.querySelector('#directory-reset').addEventListener('click', () => {
  category = 'all'; search.value = ''; filter(); search.focus();
});
addEventListener('popstate', readUrl);
readUrl();
