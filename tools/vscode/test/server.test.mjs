// tools/vscode/test/server.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { resolveBundledServer } from '../out/server.js';

const extensionDirectory = path.dirname(fileURLToPath(import.meta.url));
const temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'fer-vscode-server-'));
const executable = process.platform === 'win32' ? 'fer-lsp.exe' : 'fer-lsp';
const serverDirectory = path.join(temporaryDirectory, 'server', `${process.platform}-${process.arch}`);
const serverPath = path.join(serverDirectory, executable);

try {
  assert.equal(resolveBundledServer(temporaryDirectory), undefined);

  fs.mkdirSync(serverDirectory, { recursive: true });
  fs.writeFileSync(serverPath, 'test server');
  if (process.platform !== 'win32') fs.chmodSync(serverPath, 0o755);
  assert.equal(resolveBundledServer(temporaryDirectory), serverPath);

  if (process.platform !== 'win32') {
    fs.chmodSync(serverPath, 0o644);
    assert.equal(resolveBundledServer(temporaryDirectory), undefined);
  }
} finally {
  fs.rmSync(temporaryDirectory, { recursive: true, force: true });
}

assert.ok(extensionDirectory.endsWith(`${path.sep}test`));
