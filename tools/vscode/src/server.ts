// tools/vscode/src/server.ts

import fs from 'node:fs';
import path from 'node:path';

/** Resolve a platform-specific Fer server bundled inside the extension. */
export function resolveBundledServer(extensionPath: string): string | undefined {
  const executable = process.platform === 'win32' ? 'fer-lsp.exe' : 'fer-lsp';
  const platformDirectory = `${process.platform}-${process.arch}`;
  const candidate = path.join(extensionPath, 'server', platformDirectory, executable);
  if (!fs.existsSync(candidate)) return undefined;

  if (process.platform !== 'win32' && (fs.statSync(candidate).mode & 0o111) === 0)
    return undefined;

  return candidate;
}
