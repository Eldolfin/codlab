{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    home-manager.url = "github:nix-community/home-manager";
  };
  outputs = {
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
          inherit (inputs) self home-manager;
          inherit system;
          inherit pkgs;
        };
      }
    );
}
