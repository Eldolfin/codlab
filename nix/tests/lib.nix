test: {
  pkgs,
  self,
  system,
  home-manager,
  ...
}: let
  inherit (pkgs) lib;
  nixos-lib = import (pkgs.path + "/nixos/lib") {};
in
  (
    nixos-lib.runTest {
      inherit (test) name testScript;
      hostPkgs = pkgs;
      # This speeds up the evaluation by skipping evaluating documentation
      defaults.documentation.enable = lib.mkDefault false;
      enableOCR = true;
      node.specialArgs = {inherit self home-manager system;};
      nodes = {
        server = _: {
          imports = [./common/server.nix];
        };
        client1 = _: {
          imports = [./common/client.nix];
        };
        client2 = _: {
          imports = [./common/client.nix];
        };
      };
    }
  )
  .config
  .result
