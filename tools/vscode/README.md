# Fer VS Code support

This extension provides Fer language identification, the existing TextMate grammar, language configuration, the Fer Lavender theme, and a diagnostics-only Language Server Protocol client.

## Local development

Build the Rust language server from the repository root:

```sh
cargo build -p fer-lsp
```

Install and validate the extension dependencies from this directory:

```sh
npm install
npm run compile
npm test
```

The default `fer.server.path` is `fer-lsp`. When the extension contains a matching bundled binary under `server/<platform>-<architecture>/`, it uses that binary automatically. For local development without a bundle, set `fer.server.path` to the absolute path of the binary produced by `cargo build -p fer-lsp`, for example the path ending in `target/debug/fer-lsp` under your checkout. `fer.server.args` can provide additional server arguments when needed.

To stage a locally built server into the extension package, run the following from this directory:

```sh
FER_LSP_BINARY=../../target/debug/fer-lsp npm run prepare:server
```

This creates `server/linux-x64/fer-lsp` on the current platform and preserves executable permissions. Release automation should run the same staging command once per supported platform and architecture, using the corresponding release binary. The staging script fails fast when `FER_LSP_BINARY` is missing or does not identify a regular file.

To build a VSIX after staging the server, install the development dependencies and run:

```sh
npm run package
```

Open this directory as an extension development project in VS Code and start the Extension Development Host. Open a `.fer` document, type an undefined name such as `missing`, and confirm that the Fer diagnostic appears. Replace it with a valid expression and confirm that the diagnostic list becomes empty. The server uses full-text document synchronization and publishes English diagnostics with stable Fer diagnostic codes.

For the source below:

```fer
main = () -> i64 {
  missing
}
```

the protocol payload contains an independent message, source, and code:

```text
message: cannot resolve name missing
source: fer
code: undefined-name
```

VS Code may display these fields together as:

```text
cannot resolve name missing fer(undefined-name)
```

The `fer(undefined-name)` suffix is editor presentation of the LSP `source` and `code` fields; it is not part of the localized compiler message. The Chinese catalog currently renders the message as `无法解析名称 missing`.

The extension intentionally does not advertise document formatting yet. The current compiler lexer and HIR discard trivia and layout, so a formatter would not be able to produce lossless, safe edits. Formatting will be added after a lossless token/CST representation and formatter are available.
