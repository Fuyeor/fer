// tools/vscode/src/extension.ts

import * as vscode from 'vscode';
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

/** Start the configured Fer language server for Fer documents. */
export function activate(context: vscode.ExtensionContext): void {
  const configuration = vscode.workspace.getConfiguration('fer.server');
  const command = configuration.get<string>('path', 'fer-lsp').trim();
  const args = configuration.get<string[]>('args', []);
  if (command.length === 0) throw new Error('fer.server.path must not be empty');

  const serverOptions: ServerOptions = {
    command,
    args,
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'fer' },
      { scheme: 'untitled', language: 'fer' },
    ],
  };

  client = new LanguageClient(
    'fer-lsp',
    'Fer Language Server',
    serverOptions,
    clientOptions,
  );
  context.subscriptions.push(client);
  void client.start();
}

/** Stop the language server when the extension host is shutting down. */
export async function deactivate(): Promise<void> {
  if (client === undefined) return;
  await client.stop();
  client = undefined;
}
