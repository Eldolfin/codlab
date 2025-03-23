# Codlab

A code collaboration tool based on the language server protocol.

Goal: allowing easy cross-editor live collaboration

## TODO

### Needed for demo

- [x] server accept multiple client & broadcast change events
- [ ] draw other client cursors
- [ ] check that there is no possible race conditions that would cause a desync
      in client's documents

### Advanced features

- [ ] lsp sharing: if one client has an lsp server available but the other
      doesn't, use the available lsp server remotely
  - requesting to lsp cannot be done from an lsp, this would need to be
    implemented in an editor specific plugin

### $$?

- [ ] accounts & authentication
- [ ] vscode extension
- [ ] intellij extension
- [ ] deployment
- [ ] ssl
- [ ] frontpage?
