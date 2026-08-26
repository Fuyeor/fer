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

The default `fer.server.path` is `fer-lsp`, so the server must be available on `PATH`. For local development, set `fer.server.path` to the absolute path of the binary produced by `cargo build -p fer-lsp`, for example the path ending in `target/debug/fer-lsp` under your checkout. `fer.server.args` can provide additional server arguments when needed.

Open this directory as an extension development project in VS Code and start the Extension Development Host. Open a `.fer` document, type an undefined name such as `missing`, and confirm that the Fer diagnostic appears. Replace it with a valid expression and confirm that the diagnostic list becomes empty. The server uses full-text document synchronization and publishes English diagnostics with stable Fer diagnostic codes.

The extension intentionally does not advertise document formatting yet. The current compiler lexer and HIR discard trivia and layout, so a formatter would not be able to produce lossless, safe edits. Formatting will be added after a lossless token/CST representation and formatter are available.
