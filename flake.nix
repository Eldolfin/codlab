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
            asciinema_3
            bacon
            zellij

            # test IDEs
            helix
            neovim
            vscodium-fhs
          ];
        };
      }
    );
}
