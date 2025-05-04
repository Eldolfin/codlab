{
  self,
  lib,
  system,
  pkgs,
  ...
}: {
  programs.helix = {
    enable = true;
    languages = {
      language-server = {
        codlab = {
          command = lib.getExe' self.packages.${system}.codlab "client";
          args = ["ws://server:7575"];
        };
      };
      language = [
        {
          name = "markdown";
          auto-format = true;
          language-servers = [
            "codlab"
          ];
          formatter = {
            command = lib.getExe pkgs.deno;
            args = [
              "fmt"
              "-"
              "--ext"
              "md"
            ];
          };
        }
      ];
    };
  };
}
