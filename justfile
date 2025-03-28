test:
    git ls-files | entr -cr cargo test
server:
    git ls-files | entr -cr cargo run --bin server

demo:
    zellij -l demo/layout.kdl

demo-record:
    asciinema rec --overwrite demo/demo.cast

book:
    cd book && mdbook serve --open
