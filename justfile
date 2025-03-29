test:
    git ls-files | entr -cr cargo test

server:
    git ls-files | entr -cr cargo run --bin server

book:
    cd book && mdbook serve --open

vscode-extension:
    cd ./editors/vscode && vsce package --allow-star-activation --skip-license
