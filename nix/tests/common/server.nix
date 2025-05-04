{
  pkgs,
  self,
  system,
  ...
}: {
  # TODO: nixos service
  eldolfin.services.codlab-server.enable = true;
}
