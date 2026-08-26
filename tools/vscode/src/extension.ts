// tools/vscode/src/extension.ts

import * as vscode from 'vscode';
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
} from 'vscode-languageclient/node';

import { resolveBundledServer } from './server';

let client: LanguageClient | undefined;

/** Start the configured Fer language server for Fer documents. */
export function activate(context: vscode.ExtensionContext): void {
  const configuration = vscode.workspace.getConfiguration('fer.server');
  const configuredCommand = configuration.get<string>('path', 'fer-lsp').trim();
  const args = configuration.get<string[]>('args', []);
  if (configuredCommand.length === 0) throw new Error('fer.server.path must not be empty');

  const command =
    configuredCommand === 'fer-lsp'
      ? (resolveBundledServer(context.extensionPath) ?? configuredCommand)
      : configuredCommand;
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
