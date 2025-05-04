{lib, ...}: {
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
