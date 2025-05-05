{
  nixpkgs,
  system,
  crane,
  ...
}: let
  pkgs = nixpkgs.legacyPackages.${system};

  craneLib = crane.mkLib pkgs;
  src = craneLib.cleanCargoSource ../../.;

  # Common arguments can be set here to avoid repeating them later
  commonArgs = {
    inherit src;
    strictDeps = true;

    buildInputs = [
      # Add additional build inputs here
    ];

    # Additional environment variables can be set directly
    # MY_CUSTOM_VAR = "some value";
  };

  # Build *just* the cargo dependencies, so we can reuse
  # all of that work (e.g. via cachix) when running in CI
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  # Build the actual crate itself, reusing the dependency
  # artifacts from above.
  codlab = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
    });
in {
  inherit codlab;
  default = codlab;
}
