{
  pkgs,
  self,
  home-manager,
  system,
  ...
}: {
  imports = [
    home-manager.nixosModules.home-manager
    ./auto-login.nix
    ./wm.nix
    ./virt.nix
  ];
  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    users.alice = import ./home.nix;
    extraSpecialArgs = {
      inherit self;
      inherit system;
    };
  };
  environment.systemPackages = with pkgs; [
    self.packages.${system}.codlab
    kitty

    helix
    neovim
    vscodium-fhs
  ];

  users.users.alice = {
    isNormalUser = true;
    description = "Alice Foobar";
    password = "foobar";
    uid = 1000;
  };
}
