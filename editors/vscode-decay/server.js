const fs = require('fs');
const path = require('path');

function projectRoot(fileName) {
  if (!fileName) return undefined;
  let directory = path.dirname(path.resolve(fileName));
  while (true) {
    if (fs.existsSync(path.join(directory, 'sindri.toml'))) return directory;
    const parent = path.dirname(directory);
    if (parent === directory) return undefined;
    directory = parent;
  }
}

function serverCommand(configured, environment = process.env) {
  return environment.SINDRI_DECAY_LSP || configured || 'decay-lsp';
}

function watchedFiles() {
  return '**/*.{decay,scene.json,prefab.json,profile.json,ogg,wav,mp3}';
}

module.exports = { projectRoot, serverCommand, watchedFiles };
