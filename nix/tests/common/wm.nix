{lib, ...}: {
  # https://github.com/NixOS/nixpkgs/blob/8d92119c540d78599ba208010c722a60958810f4/nixos/tests/common/x11.nix
  services.xserver.enable = true;

  # Use IceWM as the window manager.
  # Don't use a desktop manager.
  services.displayManager.defaultSession = lib.mkDefault "none+icewm";
  services.xserver = {
    windowManager.icewm.enable = true;
    # xkb.layout = "fr";
  };
}
