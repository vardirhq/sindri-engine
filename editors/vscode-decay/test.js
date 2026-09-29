const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const {
  projectRoot,
  bundledServerName,
  bundledServerPath,
  ensureExecutable,
  serverCommand,
  watchedFiles
} = require('./server');

const manifest = require('./package.json');
const language = require('./language-configuration.json');
const grammar = require('./syntaxes/decay.tmLanguage.json');

assert.deepStrictEqual(manifest.contributes.languages[0].extensions, ['.decay']);
assert.strictEqual(manifest.contributes.grammars[0].scopeName, 'source.decay');
assert.strictEqual(language.comments.lineComment, '//');
assert.ok(language.indentationRules.increaseIndentPattern);
assert.strictEqual(grammar.scopeName, 'source.decay');

function scopeFor(source) {
  const scopes = [];
  for (const group of Object.values(grammar.repository)) {
    for (const pattern of group.patterns || []) {
      if (pattern.match && new RegExp(pattern.match, 'g').test(source)) {
        scopes.push(pattern.name || Object.values(pattern.captures || {}).map(value => value.name).join(' '));
      }
    }
  }
  return scopes.join(' ');
}

const representative = `@export let speed: f32 = 2.0;
event Goal(player: Entity);
state Game { var score: f32 = 0; }
shared fn half(size: f32) -> f32 { return size * 0.5; }
enum Phase { Lobby, Play }
struct Card { name: String, weight: f32 }
shared const LIMIT: f32 = 3.0;
script Player { on Goal(player: Entity) { this.flash(); } fn flash() { for item in items { if true { item.emit(); } } } }`;
const scopes = scopeFor(representative);
for (const expected of ['annotation', 'shared', 'enum', 'struct', 'const', 'event', 'state', 'type', 'function', 'control', 'numeric', 'this', 'member', 'operator']) {
  assert.ok(scopes.includes(expected), `representative syntax lacks ${expected} scope`);
}

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'decay-vscode-'));
const root = path.join(temporary, 'game');
const script = path.join(root, 'assets', 'scripts', 'player.decay');
const extension = path.join(temporary, 'extension');
fs.mkdirSync(path.dirname(script), { recursive: true });
fs.mkdirSync(path.join(extension, 'bin'), { recursive: true });
fs.writeFileSync(path.join(root, 'sindri.toml'), '[project]\nname = "Test"\n');
fs.writeFileSync(script, representative);
assert.strictEqual(projectRoot(script), root);
assert.strictEqual(projectRoot(path.join(temporary, 'outside.decay')), undefined);
assert.strictEqual(bundledServerName('linux'), 'decay-lsp');
assert.strictEqual(bundledServerName('win32'), 'decay-lsp.exe');
assert.strictEqual(bundledServerPath(extension, 'linux'), path.join(extension, 'bin', 'decay-lsp'));

const linuxServer = bundledServerPath(extension, 'linux');
fs.writeFileSync(linuxServer, 'test', { mode: 0o644 });
assert.strictEqual(serverCommand('decay-lsp', {}, extension, 'linux'), linuxServer);
let chmodCall;
ensureExecutable(linuxServer, 'linux', (file, mode) => { chmodCall = { file, mode }; });
assert.deepStrictEqual(chmodCall, { file: linuxServer, mode: 0o755 });
chmodCall = undefined;
ensureExecutable(linuxServer, 'win32', () => { throw new Error('Windows must not chmod'); });
assert.strictEqual(chmodCall, undefined);
assert.strictEqual(serverCommand('/custom/decay-lsp', {}, extension, 'linux'), '/custom/decay-lsp');
assert.strictEqual(serverCommand('decay-lsp', { SINDRI_DECAY_LSP: '/env/decay-lsp' }, extension, 'linux'), '/env/decay-lsp');
assert.strictEqual(serverCommand('decay-lsp', {}, '/missing', 'linux', () => false), 'decay-lsp');
assert.ok(watchedFiles().includes('decay'));
fs.rmSync(temporary, { recursive: true, force: true });

console.log('Decay VS Code extension checks passed');
