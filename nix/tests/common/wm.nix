{lib, ...}: {
  services = {
    # https://github.com/NixOS/nixpkgs/blob/8d92119c540d78599ba208010c722a60958810f4/nixos/tests/common/x11.nix
    xserver = {
      enable = true;
      windowManager.icewm.enable = true;
      # xkb.layout = "fr";
    };

    # Use IceWM as the window manager.
    # Don't use a desktop manager.
    displayManager.defaultSession = lib.mkDefault "none+icewm";
  };
}
