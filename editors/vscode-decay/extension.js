const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');
const { projectRoot, serverCommand, watchedFiles } = require('./server');

let client;

async function activate(context) {
  const configured = vscode.workspace.getConfiguration('decay').get('server.path', 'decay-lsp');
  const command = serverCommand(configured, process.env, context.extensionPath);
  const activeFile = vscode.window.activeTextEditor?.document.fileName;
  const root = projectRoot(activeFile);
  const workspaceFolder = root
    ? { uri: vscode.Uri.file(root), name: vscode.workspace.name || 'Sindri project', index: 0 }
    : vscode.window.activeTextEditor
      ? vscode.workspace.getWorkspaceFolder(vscode.window.activeTextEditor.document.uri)
      : undefined;
  const watcher = vscode.workspace.createFileSystemWatcher(watchedFiles());
  context.subscriptions.push(watcher);
  const serverOptions = {
    command,
    args: []
  };
  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'decay' }],
    workspaceFolder,
    synchronize: {
      fileEvents: watcher
    }
  };
  client = new LanguageClient('decay', 'Decay Language Server', serverOptions, clientOptions);
  try {
    await client.start();
    context.subscriptions.push(client);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(
      `Decay language server could not start (${command}): ${detail}. ` +
      'Reinstall the Sindri Decay extension, or override the server with SINDRI_DECAY_LSP / decay.server.path.'
    );
  }
}

async function deactivate() {
  if (client) {
    await client.stop();
  }
}

module.exports = { activate, deactivate };
