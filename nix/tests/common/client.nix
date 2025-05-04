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
    wezterm

    helix
    neovim
    vscodium-fhs
  ];

  virtualisation = {
    memorySize = 4096;
    diskSize = 8192;
    cores = 6;
    resolution = {
      x = 1920;
      y = 1080;
    };
  };
  users.users.alice = {
    isNormalUser = true;
    description = "Alice Foobar";
    password = "foobar";
    uid = 1000;
  };
}
