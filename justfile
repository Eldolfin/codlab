# List available recipes
default:
    @just --list

# Run the rust tests, restart on change
test:
    git ls-files | entr -cr cargo test

# Run the server, restart on change
server:
    git ls-files | entr -cr cargo run --bin server

# Open a live-reloading web view of the doc
book:
    cd book && mdbook serve --open

# Build vscode extension at `editors/vscode/lsp-sample-1.0.0.vsix`
vscode-extension:
    cd ./editors/vscode && vsce package --allow-star-activation --skip-license

# Debug a nixos test interactively
nixos-test-interactive:
    nix build .#checks.x86_64-linux.simple.driverInteractive && ./result/bin/nixos-test-driver

# Runs all nix tests one by one
ci:
    #!/usr/bin/env bash
    set -e
    TESTS=$(nix flake show --all-systems --json | jq -r '.checks."x86_64-linux" | keys[]')
    printf "\033[1;34mRunning tests: \033[1;33m%s\033[0m\n" "[$(echo "$TESTS" | paste -sd, -)]"
    for test in $TESTS; do
        printf "\033[1;34mRunning test \033[1;33m%s\033[0m\n" "$test"
        nix build -L .#checks.x86_64-linux.$test
    done
