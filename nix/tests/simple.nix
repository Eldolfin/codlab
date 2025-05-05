(import ./lib.nix) ({lib, ...}: {
  name = "simple";
  testScript = lib.readFile ./simple.py;
})
