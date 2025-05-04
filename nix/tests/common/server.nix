{
  self,
  system,
  ...
}: {
  imports = [
    self.nixosModules.${system}.default
    ./virt.nix
  ];

  eldolfin.services.codlab-server = {
    enable = true;
    port = 7575;
  };
}
