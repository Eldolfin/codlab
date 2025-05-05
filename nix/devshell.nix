{
  pkgs,
  self,
  system,
  ...
}:
pkgs.mkShell {
  inherit (self.checks.${system}.pre-commit-check) shellHook;
  buildInputs = with pkgs; [
    self.checks.${system}.pre-commit-check.enabledPackages

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
