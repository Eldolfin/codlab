{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };
  outputs = {
    self,
    nixpkgs,
    utils,
    ...
  } @ inputs:
    utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShell = import ./nix/devshell.nix {
          inherit pkgs;
        };
        nixosModules = import ./nix/nixosModules {
          inherit (inputs) nixpkgs self;
          inherit system;
        };
        packages = import ./nix/pkgs {
          inherit (inputs) nixpkgs crane;
          inherit system;
        };
        checks = import ./nix/tests {
          inherit self;
          inherit system;
          pkgs = nixpkgs.legacyPackages.${system};
        };
      }
    );
}
