CI_OUTPUT := "ci-output"

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
    rm -rf {{CI_OUTPUT}} && mkdir -p {{CI_OUTPUT}}
    TESTS=$(nix flake show --all-systems --json | jq -r '.checks."x86_64-linux" | keys[]')
    printf "{{BLUE}}Running tests: {{YELLOW}}[$(echo "$TESTS" | paste -sd, -)]{{NORMAL}}\n"
    for test in $TESTS; do
        printf "{{BLUE}}Running test {{YELLOW}}%s{{NORMAL}}\n" "$test"
        nix build -L .#checks.x86_64-linux.$test
        if [ "$test" != "pre-commit-check" ]; then
            printf "{{BLUE}}Concatenating videos of clients{{NORMAL}}\n"
            find result/client* -name '*.mkv' |
                sort |
                xargs -I{} echo -i {} |
                xargs ffmpeg -filter_complex vstack {{CI_OUTPUT}}/$test.mp4
        fi
    done
