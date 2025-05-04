{
  pkgs,
  lib,
  self,
  ...
}: {
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
  # https://github.com/NixOS/nixpkgs/blob/8d92119c540d78599ba208010c722a60958810f4/nixos/tests/common/x11.nix
  services.xserver.enable = true;

  # Use IceWM as the window manager.
  # Don't use a desktop manager.
  services.displayManager.defaultSession = lib.mkDefault "none+icewm";
  services.xserver.windowManager.icewm.enable = true;

  services.xserver.displayManager.lightdm.enable = true;
  services.displayManager.autoLogin = {
    enable = true;
    user = "alice";
  };
  # https://github.com/NixOS/nixpkgs/blob/7b616e2913410e7b9cf549c2ee58bbbd3033d826/nixos/tests/common/auto.nix
  # lightdm by default doesn't allow auto login for root, which is
  # required by some nixos tests. Override it here.
  security.pam.services.lightdm-autologin.text = lib.mkForce ''
    auth     requisite pam_nologin.so
    auth     required  pam_succeed_if.so quiet
    auth     required  pam_permit.so

    account  include   lightdm

    password include   lightdm

    session  include   lightdm
  '';
}
