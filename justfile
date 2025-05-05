test:
    git ls-files | entr -cr cargo test

server:
    git ls-files | entr -cr cargo run --bin server

book:
    cd book && mdbook serve --open

vscode-extension:
    cd ./editors/vscode && vsce package --allow-star-activation --skip-license

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
        nix build .#checks.x86_64-linux.$test
    done

