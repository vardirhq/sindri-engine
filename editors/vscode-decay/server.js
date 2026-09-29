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

function bundledServerName(platform = process.platform) {
  return platform === 'win32' ? 'decay-lsp.exe' : 'decay-lsp';
}

function bundledServerPath(extensionPath, platform = process.platform) {
  if (!extensionPath) return undefined;
  return path.join(extensionPath, 'bin', bundledServerName(platform));
}

function serverCommand(configured, environment = process.env, extensionPath, platform = process.platform, exists = fs.existsSync) {
  if (environment.SINDRI_DECAY_LSP) return environment.SINDRI_DECAY_LSP;

  // An explicit non-default setting is an intentional override. The default
  // remains `decay-lsp` for source-tree development, but a packaged extension
  // should be zero-setup and prefer the binary shipped beside it.
  if (configured && configured !== 'decay-lsp') return configured;

  const bundled = bundledServerPath(extensionPath, platform);
  if (bundled && exists(bundled)) return bundled;

  return configured || 'decay-lsp';
}

function watchedFiles() {
  return '**/*.{decay,scene.json,prefab.json,profile.json,ogg,wav,mp3}';
}

module.exports = {
  projectRoot,
  bundledServerName,
  bundledServerPath,
  serverCommand,
  watchedFiles
};
