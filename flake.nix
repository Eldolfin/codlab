{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    utils,
  }:
    utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc

            # tools
            act
            bacon
            mdbook
            vsce
            zellij

            # test IDEs
            helix
            neovim
            vscodium-fhs
          ];
        };
        checks = import ./nix/tests {
          pkgs = nixpkgs.legacyPackages.${system};
        };
      }
    );
}
