{
  pre-commit-hooks,
  system,
  ...
}:
pre-commit-hooks.lib.${system}.run {
  src = ./.;
  hooks = {
    # formatters
    alejandra.enable = true;
    rustfmt.enable = true;
    end-of-file-fixer.enable = true;

    # linters
    actionlint.enable = true;
    statix.enable = true;
    check-merge-conflicts.enable = true;
    clippy = {
      enable = true;
      settings = {
        allFeatures = true;
      };
    };
    deadnix.enable = true;
    markdownlint = {
      enable = true;
      settings.configuration = {
        MD013 = false; # line lenght check
        MD034 = false; # bare links for demo video in readme
      };
    };
    nil.enable = true;
    ripsecrets.enable = true;
  };
}
