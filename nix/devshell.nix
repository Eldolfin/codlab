{pkgs, ...}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    rustc

    # tools
    act
    bacon
    just
    mdbook
    vsce
    zellij

    # test IDEs
    helix
    neovim
    vscodium-fhs
  ];
}
