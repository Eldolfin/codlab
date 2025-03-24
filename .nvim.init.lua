vim.lsp.start({
  name = 'codlab',
  cmd = { 'cargo', 'run', '-q', '--bin', 'client' },
  filetypes = { 'markdown' },
})
