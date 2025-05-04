{self, ...}: let
  nixosModule = {
    config,
    lib,
    pkgs,
    ...
  }:
    with lib; let
      cfg = config.eldolfin.services.codlab-server;
    in {
      options.eldolfin.services.codlab-server = {
        enable = mkEnableOption "Enables the codlab-server service";
        # TODO: allow server port to be configurable
        port = mkOption {
          type = types.port;
          default = 7575;
          example = 7575;
          description = "The server port to open (firewall will be opened)";
        };
      };

      config =
        mkIf cfg.enable
        {
          networking = {
            firewall.allowedTCPPorts = [cfg.port];
          };
          systemd.services."eldolfin.codlab-server" = {
            wantedBy = ["multi-user.target"];
            environment = {
              RUST_LOG = "debug";
            };

            serviceConfig = let
              pkg = self.packages.${pkgs.system}.codlab;
            in {
              Restart = "always";
              RestartSec = 2;
              ExecStart = "!${pkg}/bin/server";
              RuntimeDirectory = "eldolfin.codlab-server";
              RuntimeDirectoryMode = "0755";
              StateDirectory = "eldolfin.codlab-server";
              StateDirectoryMode = "0700";
              CacheDirectory = "eldolfin.codlab-server";
              CacheDirectoryMode = "0750";
            };
          };
        };
    };
in {
  default = nixosModule;
}
