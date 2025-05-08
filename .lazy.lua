local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Register the LSP if it's not already defined
if not configs.codlab then
  configs.codlab = {
    default_config = {
      cmd = { 'cargo', 'run', '-q', '--bin', 'client', '--', 'ws://127.0.0.1:7575' },
      filetypes = { '*' },
      root_dir = lspconfig.util.root_pattern("Cargo.toml", ".git"),
      settings = {}, -- Add any server-specific settings here
    },
  }
end

-- Start the server
lspconfig.codlab.setup({})
return {}
