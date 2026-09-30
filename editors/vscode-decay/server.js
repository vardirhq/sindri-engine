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

function ensureExecutable(command, platform = process.platform, chmod = fs.chmodSync) {
  if (!command || platform === 'win32') return;
  // VSIX/ZIP extraction does not reliably preserve Unix executable bits. The
  // bundled LSP is data until we restore that bit after installation.
  try {
    chmod(command, 0o755);
  } catch (error) {
    const code = error && typeof error === 'object' ? error.code : undefined;
    if (code !== 'ENOENT') throw error;
  }
}

function serverCommand(configured, environment = process.env, extensionPath, platform = process.platform, exists = fs.existsSync) {
  if (environment.SINDRI_DECAY_LSP) return environment.SINDRI_DECAY_LSP;

  if (configured && configured !== 'decay-lsp') return configured;

  const bundled = bundledServerPath(extensionPath, platform);
  if (bundled && exists(bundled)) return bundled;

  return configured || 'decay-lsp';
}

function watchedFiles() {
  return '**/*.{decay,scene,prefab,profile,ogg,wav,mp3}';
}

module.exports = {
  projectRoot,
  bundledServerName,
  bundledServerPath,
  ensureExecutable,
  serverCommand,
  watchedFiles
};
