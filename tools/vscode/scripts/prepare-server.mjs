// tools/vscode/scripts/prepare-server.mjs

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const extensionDirectory = path.resolve(scriptDirectory, '..');
const input = process.env.FER_LSP_BINARY;
if (input === undefined || input.trim().length === 0)
  throw new Error('FER_LSP_BINARY must point to a built fer-lsp executable');

const inputPath = path.resolve(input);
if (!fs.existsSync(inputPath)) throw new Error(`Fer server does not exist: ${inputPath}`);
if (!fs.statSync(inputPath).isFile()) throw new Error(`Fer server is not a file: ${inputPath}`);

const executable = process.platform === 'win32' ? 'fer-lsp.exe' : 'fer-lsp';
const targetDirectory = path.join(extensionDirectory, 'server', `${process.platform}-${process.arch}`);
const targetPath = path.join(targetDirectory, executable);
fs.mkdirSync(targetDirectory, { recursive: true });
fs.copyFileSync(inputPath, targetPath);
if (process.platform !== 'win32') fs.chmodSync(targetPath, 0o755);

process.stdout.write(`Prepared ${targetPath}${os.EOL}`);
