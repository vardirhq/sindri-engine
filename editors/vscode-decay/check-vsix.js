// Loads extension.js from a packaged VSIX, not the source tree, so a runtime
// dependency missing from the package fails here instead of silently disabling
// the language server in VS Code while syntax highlighting keeps working.
const assert = require('assert');
const fs = require('fs');
const Module = require('module');
const os = require('os');
const path = require('path');
const zlib = require('zlib');

function extract(vsix, destination) {
  const zip = fs.readFileSync(vsix);
  const end = zip.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  assert.ok(end >= 0, `${vsix} is not a zip archive`);
  const count = zip.readUInt16LE(end + 10);
  let entry = zip.readUInt32LE(end + 16);
  for (let index = 0; index < count; index += 1) {
    const method = zip.readUInt16LE(entry + 10);
    const size = zip.readUInt32LE(entry + 20);
    const nameLength = zip.readUInt16LE(entry + 28);
    const extraLength = zip.readUInt16LE(entry + 30);
    const commentLength = zip.readUInt16LE(entry + 32);
    const local = zip.readUInt32LE(entry + 42);
    const name = zip.toString('utf8', entry + 46, entry + 46 + nameLength);
    entry += 46 + nameLength + extraLength + commentLength;
    if (name.endsWith('/')) continue;
    const start = local + 30 + zip.readUInt16LE(local + 26) + zip.readUInt16LE(local + 28);
    const data = zip.subarray(start, start + size);
    const target = path.join(destination, name);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, method === 8 ? zlib.inflateRawSync(data) : data);
  }
}

const vsix = path.resolve(process.argv[2] || 'sindri-decay.vsix');
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'decay-vsix-'));
try {
  extract(vsix, temporary);
  const stub = path.join(temporary, 'vscode-stub.js');
  // Every vscode API member is a constructible, callable placeholder: loading
  // only needs the module graph to resolve, not a working editor.
  fs.writeFileSync(stub, `const stub = new Proxy(function () {}, {
  get: (target, key) => key === '__esModule' ? false : stub,
  apply: () => stub,
  construct: () => stub
});
module.exports = stub;
`);
  const resolve = Module._resolveFilename;
  Module._resolveFilename = function (request, ...rest) {
    return request === 'vscode' ? stub : resolve.call(this, request, ...rest);
  };
  const extension = require(path.join(temporary, 'extension', 'extension.js'));
  assert.strictEqual(typeof extension.activate, 'function');
  console.log(`Packaged Decay extension loads from ${path.basename(vsix)}`);
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
